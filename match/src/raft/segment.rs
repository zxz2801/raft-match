//! Raft segment implementation
//! This module provides a file-based segment implementation for storing Raft entries.
//! Each segment contains a range of entries and maintains an index of their positions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

const HEADER_SIZE: u64 = 16; // 8 bytes for start_index + 8 bytes for end_index
const ENTRY_HEADER_SIZE: u64 = 8; // 8 bytes for entry size

/// Represents a segment of Raft entries stored in a file
/// Each segment contains a range of entries and maintains their positions
#[derive(Debug)]
pub struct Segment {
    file: File,                          // The underlying file
    start_index: u64,                    // The index of the first entry in this segment
    end_index: u64,                      // The index of the last entry in this segment
    path: String,                        // Path to the segment file
    entry_positions: BTreeMap<u64, u64>, // Maps entry index to its position in the file
}

/// Header information stored at the beginning of each segment file
#[derive(Debug, Serialize, Deserialize)]
struct SegmentHeader {
    start_index: u64, // The index of the first entry in this segment
    end_index: u64,   // The index of the last entry in this segment
}

impl Segment {
    /// Create a new segment or open an existing one
    /// Initializes the segment file and reads its header if it exists
    pub fn new<P: AsRef<Path>>(path: P, start_index: u64) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
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

    /// Write the segment header to the file
    fn write_header(&mut self) -> io::Result<()> {
        let header = SegmentHeader {
            start_index: self.start_index,
            end_index: self.end_index,
        };

        let header_bytes = bincode::serialize(&header).map_err(io::Error::other)?;

        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&header_bytes)?;
        Ok(())
    }

    /// Read the segment header from the file
    fn read_header(&mut self) -> io::Result<()> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut header_bytes = vec![0u8; HEADER_SIZE as usize];
        self.file.read_exact(&mut header_bytes)?;

        let header: SegmentHeader =
            bincode::deserialize(&header_bytes).map_err(io::Error::other)?;

        self.start_index = header.start_index;
        self.end_index = header.end_index;
        Ok(())
    }

    /// Write an entry header containing its size
    fn write_entry_header(&mut self, size: u64) -> io::Result<()> {
        let size_bytes = size.to_le_bytes();
        self.file.write_all(&size_bytes)?;
        Ok(())
    }

    /// Read an entry header to get its size
    fn read_entry_header(&mut self) -> io::Result<u64> {
        let mut size_bytes = [0u8; 8];
        self.file.read_exact(&mut size_bytes)?;
        Ok(u64::from_le_bytes(size_bytes))
    }

    /// Rebuild the entry position index by scanning the file
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

    /// Append new entries to the segment
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

    /// Read an entry at the specified index
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

    /// Clear the segment by removing its file
    pub fn clear(&mut self) -> io::Result<()> {
        std::fs::remove_file(&self.path)?;
        Ok(())
    }

    /// Get the start index of this segment
    #[allow(unused)]
    pub fn get_start_index(&self) -> u64 {
        self.start_index
    }

    /// Get the end index of this segment
    pub fn get_end_index(&self) -> u64 {
        self.end_index
    }

    /// Check if the segment is empty
    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        self.end_index <= self.start_index
    }
}
