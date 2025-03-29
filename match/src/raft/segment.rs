use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

const HEADER_SIZE: u64 = 16; // 8 bytes for start_index + 8 bytes for end_index
const ENTRY_HEADER_SIZE: u64 = 8; // 8 bytes for entry size

#[derive(Debug)]
pub struct Segment {
    file: File,
    start_index: u64,
    end_index: u64,
    path: String,
    entry_positions: BTreeMap<u64, u64>, // index -> file position
}

#[derive(Debug, Serialize, Deserialize)]
struct SegmentHeader {
    start_index: u64,
    end_index: u64,
}

impl Segment {
    pub fn new<P: AsRef<Path>>(path: P, start_index: u64) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        let mut segment = Segment {
            file,
            start_index,
            end_index: start_index,
            path: path.as_ref().to_string_lossy().to_string(),
            entry_positions: BTreeMap::new(),
        };

        // Initialize header if file is empty
        if segment.file.metadata()?.len() == 0 {
            segment.write_header()?;
        } else {
            segment.read_header()?;
            segment.rebuild_entry_positions()?;
        }

        Ok(segment)
    }

    fn write_header(&mut self) -> io::Result<()> {
        let header = SegmentHeader {
            start_index: self.start_index,
            end_index: self.end_index,
        };

        let header_bytes =
            bincode::serialize(&header).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&header_bytes)?;
        Ok(())
    }

    fn read_header(&mut self) -> io::Result<()> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut header_bytes = vec![0u8; HEADER_SIZE as usize];
        self.file.read_exact(&mut header_bytes)?;

        let header: SegmentHeader = bincode::deserialize(&header_bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.start_index = header.start_index;
        self.end_index = header.end_index;
        Ok(())
    }

    fn write_entry_header(&mut self, size: u64) -> io::Result<()> {
        let size_bytes = size.to_le_bytes();
        self.file.write_all(&size_bytes)?;
        Ok(())
    }

    fn read_entry_header(&mut self) -> io::Result<u64> {
        let mut size_bytes = [0u8; 8];
        self.file.read_exact(&mut size_bytes)?;
        Ok(u64::from_le_bytes(size_bytes))
    }

    fn rebuild_entry_positions(&mut self) -> io::Result<()> {
        self.entry_positions.clear();
        let mut pos = HEADER_SIZE;

        while pos < self.file.metadata()?.len() {
            self.file.seek(SeekFrom::Start(pos))?;
            let entry_size = self.read_entry_header()?;
            let entry_index = self.start_index + (self.entry_positions.len() as u64);
            self.entry_positions.insert(entry_index, pos);
            pos += ENTRY_HEADER_SIZE + entry_size;
        }

        Ok(())
    }

    pub fn append(&mut self, entries: &Vec<Vec<u8>>) -> io::Result<()> {
        self.file.seek(SeekFrom::End(0))?;

        for entry in entries {
            let entry_size = entry.len() as u64;
            self.write_entry_header(entry_size)?;
            self.file.write_all(entry)?;

            let entry_index = self.end_index + 1;
            self.entry_positions.insert(
                entry_index,
                self.file.metadata()?.len() - entry_size - ENTRY_HEADER_SIZE,
            );
            self.end_index = entry_index;
        }

        self.write_header()?;
        Ok(())
    }

    pub fn read_entry(&mut self, index: u64) -> io::Result<Vec<u8>> {
        if index < self.start_index || index > self.end_index {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Index out of range",
            ));
        }

        let pos = self.entry_positions.get(&index).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Entry position not found")
        })?;

        self.file.seek(SeekFrom::Start(*pos))?;
        let entry_size = self.read_entry_header()?;

        let mut entry = vec![0u8; entry_size as usize];
        self.file.read_exact(&mut entry)?;
        Ok(entry)
    }

    pub fn truncate(&mut self, index: u64) -> io::Result<()> {
        if index < self.start_index || index > self.end_index {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Index out of range",
            ));
        }

        if let Some(pos) = self.entry_positions.get(&index) {
            self.file.set_len(*pos)?;
            self.end_index = index;
            self.write_header()?;

            // Remove entries after the truncation point
            while let Some((&idx, _)) = self.entry_positions.range(index + 1..).next() {
                self.entry_positions.remove(&idx);
            }
        }

        Ok(())
    }

    pub fn get_start_index(&self) -> u64 {
        self.start_index
    }

    pub fn get_end_index(&self) -> u64 {
        self.end_index
    }

    pub fn is_empty(&self) -> bool {
        self.end_index < self.start_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_segment_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut segment = Segment::new(temp_file.path(), 1).unwrap();

        assert_eq!(segment.get_start_index(), 1);
        assert_eq!(segment.get_end_index(), 0);
        assert!(segment.is_empty());
    }

    #[test]
    fn test_segment_append() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut segment = Segment::new(temp_file.path(), 1).unwrap();

        let entries = vec![b"first entry", b"second entry"];
        segment.append(&entries).unwrap();

        assert_eq!(segment.get_end_index(), 2);
        assert!(!segment.is_empty());

        let first_entry = segment.read_entry(1).unwrap();
        assert_eq!(first_entry, b"first entry");

        let second_entry = segment.read_entry(2).unwrap();
        assert_eq!(second_entry, b"second entry");
    }

    #[test]
    fn test_segment_truncate() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut segment = Segment::new(temp_file.path(), 1).unwrap();

        let entries = vec![b"first entry", b"second entry", b"third entry"];
        segment.append(&entries).unwrap();

        segment.truncate(2).unwrap();
        assert_eq!(segment.get_end_index(), 2);

        let first_entry = segment.read_entry(1).unwrap();
        assert_eq!(first_entry, b"first entry");

        let second_entry = segment.read_entry(2).unwrap();
        assert_eq!(second_entry, b"second entry");

        assert!(segment.read_entry(3).is_err());
    }
}
