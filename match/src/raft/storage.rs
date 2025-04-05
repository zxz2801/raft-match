//! Raft storage implementation
//! This module provides a file-based storage implementation for Raft, supporting
//! persistent storage of Raft entries, snapshots, and state.

use crate::raft::segment::Segment;
use prost::bytes::Bytes;
use protobuf::Message;
use raft::eraftpb::Entry;
use raft::eraftpb::HardState;
use raft::eraftpb::Snapshot;
use raft::{storage::MemStorage, GetEntriesContext, RaftState, Result, Storage};
use raft_proto::eraftpb::ConfState;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// File-based storage implementation for Raft
/// Combines in-memory storage with persistent file storage
pub struct FileStorage {
    mem_storage: MemStorage,          // In-memory storage for quick access
    segments: BTreeMap<u64, Segment>, // File segments for persistent storage
    base_path: PathBuf,               // Base directory for storing files
}

impl FileStorage {
    /// Create a new FileStorage instance
    /// Initializes storage from existing files or creates new storage if bootstrap is true
    pub fn new<P: AsRef<Path>>(base_path: P, bootstrap: bool) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        fs::create_dir_all(&base_path)?;

        // Initialize mem_storage
        let mem_storage = MemStorage::new();

        // Try to load snapshot if exists
        let snapshot_path = base_path.join("snapshot");
        if snapshot_path.exists() {
            let snapshot_data = fs::read(&snapshot_path)
                .map_err(|e| raft::Error::Store(raft::StorageError::Other(Box::new(e))))?;
            let mut snapshot = Snapshot::default();
            snapshot
                .merge_from_bytes(&snapshot_data)
                .map_err(|e| raft::Error::Store(raft::StorageError::Other(Box::new(e))))?;

            // Apply snapshot to mem_storage
            mem_storage.wl().apply_snapshot(snapshot)?;
        } else if bootstrap {
            // Initialize snapshot
            let mut snapshot = Snapshot::default();
            snapshot.mut_metadata().index = 1;
            snapshot.mut_metadata().term = 1;
            snapshot.mut_metadata().mut_conf_state().voters = vec![1];

            mem_storage.wl().apply_snapshot(snapshot).unwrap();
        }

        // Load all segment files
        let mut segments = BTreeMap::new();
        let mut entries = Vec::new();

        // Find all segment files
        let mut segment_files: Vec<_> = fs::read_dir(&base_path)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_file() && path.to_string_lossy().contains("segment_") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        // Sort segments by start index
        segment_files.sort_by(|a, b| {
            let a_idx = a
                .file_name()
                .unwrap()
                .to_string_lossy()
                .trim_start_matches("segment_")
                .trim_end_matches(".log")
                .parse::<u64>()
                .unwrap();
            let b_idx = b
                .file_name()
                .unwrap()
                .to_string_lossy()
                .trim_start_matches("segment_")
                .trim_end_matches(".log")
                .parse::<u64>()
                .unwrap();
            a_idx.cmp(&b_idx)
        });

        let last_index = mem_storage.last_index().unwrap();

        // Load each segment
        for segment_path in segment_files {
            let start_index = segment_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .trim_start_matches("segment_")
                .trim_end_matches(".log")
                .parse::<u64>()
                .unwrap();

            let mut segment = Segment::new(&segment_path, start_index)
                .map_err(|e| raft::Error::Store(raft::StorageError::Other(Box::new(e))))?;

            // Read all entries from this segment
            let mut current_index = start_index;
            while let Ok(entry_data) = segment.read_entry(current_index) {
                let mut entry = Entry::default();
                entry
                    .merge_from_bytes(&entry_data)
                    .map_err(|e| raft::Error::Store(raft::StorageError::Other(Box::new(e))))?;
                if entry.index > last_index {
                    entries.push(entry);
                }
                current_index += 1;
            }

            segments.insert(start_index, segment);
        }

        // Apply entries to mem_storage
        if !entries.is_empty() {
            mem_storage.wl().append(&entries)?;
        }

        Ok(FileStorage {
            mem_storage,
            segments,
            base_path,
        })
    }

    /// Get the path for a segment file
    fn get_segment_path(&self, start_index: u64) -> PathBuf {
        self.base_path.join(format!("segment_{}.log", start_index))
    }

    /// Get an existing segment or create a new one
    fn get_or_create_segment(&mut self, start_index: u64) -> Result<&mut Segment> {
        if !self.segments.contains_key(&start_index) {
            let path = self.get_segment_path(start_index);
            let segment = Segment::new(path, start_index)
                .map_err(|e| raft::Error::Store(raft::StorageError::Other(Box::new(e))))?;
            self.segments.insert(start_index, segment);
        }
        Ok(self.segments.get_mut(&start_index).unwrap())
    }

    /// Append entries to storage
    /// Writes entries to both memory and persistent storage
    pub fn append_entries(&mut self, entries: &[Entry]) -> Result<()> {
        // First append to mem_storage
        self.mem_storage.wl().append(entries)?;

        // Group entries by their segment
        let mut entries_by_segment: BTreeMap<u64, Vec<Vec<u8>>> = BTreeMap::new();

        for entry in entries {
            let segment_start = (entry.index / 10000) * 10000; // Each segment contains 1000 entries
            entries_by_segment
                .entry(segment_start)
                .or_default()
                .push(entry.write_to_bytes().unwrap());
        }

        // Append entries to their respective segments
        for (segment_start, entries) in entries_by_segment {
            let segment = self.get_or_create_segment(segment_start)?;
            segment
                .append(&entries)
                .map_err(|e| raft::Error::Store(raft::StorageError::Other(Box::new(e))))?;
        }

        Ok(())
    }

    /// Set the configuration state
    pub fn set_conf_state(&mut self, conf_state: ConfState) {
        self.mem_storage.wl().set_conf_state(conf_state)
    }

    /// Set the hard state
    pub fn set_hardstate(&mut self, hs: HardState) {
        self.mem_storage.wl().set_hardstate(hs);
    }

    /// Set the commit index
    pub fn set_commit(&mut self, commit: u64) {
        self.mem_storage.wl().mut_hard_state().set_commit(commit);
    }

    /// Apply a snapshot to storage
    /// Writes the snapshot to disk and updates memory state
    pub fn apply_snapshot(&mut self, snapshot: &Snapshot) -> Result<()> {
        let snapshot_path = self
            .base_path
            .join(format!("snapshot_{}", snapshot.get_metadata().index));
        let snapshot_data = snapshot
            .write_to_bytes()
            .map_err(|e| raft::Error::Store(raft::StorageError::Other(Box::new(e))))?;

        fs::write(&snapshot_path, &snapshot_data)
            .map_err(|e| raft::Error::Store(raft::StorageError::Other(Box::new(e))))?;
        self.mem_storage.wl().apply_snapshot(snapshot.clone())?;
        Ok(())
    }

    /// Save a snapshot of the current state
    /// Creates a new snapshot with the given business data and applied index
    pub fn save_snapshot(&mut self, biz_data: Vec<u8>, applied: u64) -> Result<()> {
        let mut snapshot = self.snapshot(applied, 0)?;
        snapshot.set_data(Bytes::from(biz_data));
        let snapshot_path = self.base_path.join("snapshot");
        let temp_path = self.base_path.join("snapshot.tmp");

        // Write to temporary file first
        let snapshot_data = snapshot
            .write_to_bytes()
            .map_err(|e| raft::Error::Store(raft::StorageError::Other(Box::new(e))))?;

        fs::write(&temp_path, &snapshot_data)
            .map_err(|e| raft::Error::Store(raft::StorageError::Other(Box::new(e))))?;

        // Remove old snapshot if exists
        if snapshot_path.exists() {
            fs::remove_file(&snapshot_path)
                .map_err(|e| raft::Error::Store(raft::StorageError::Other(Box::new(e))))?;
        }

        // Rename temp file to actual snapshot file
        fs::rename(&temp_path, &snapshot_path)
            .map_err(|e| raft::Error::Store(raft::StorageError::Other(Box::new(e))))?;

        self.mem_storage
            .wl()
            .compact(snapshot.get_metadata().index)
            .unwrap();
        let mut to_remove = Vec::new();
        for (start_index, segment) in self.segments.iter_mut() {
            if segment.get_end_index() <= snapshot.get_metadata().index {
                segment.clear()?;
                to_remove.push(*start_index);
            }
        }
        for start_index in to_remove {
            self.segments.remove(&start_index);
        }
        Ok(())
    }

    /// Get the current commit index
    pub fn commit(&self) -> u64 {
        self.mem_storage.rl().hard_state().commit
    }
}

impl Storage for FileStorage {
    /// Get the initial Raft state
    fn initial_state(&self) -> Result<RaftState> {
        self.mem_storage.initial_state()
    }

    /// Get entries from storage
    fn entries(
        &self,
        low: u64,
        high: u64,
        max_size: impl Into<Option<u64>>,
        context: GetEntriesContext,
    ) -> Result<Vec<Entry>> {
        self.mem_storage.entries(low, high, max_size, context)
    }

    /// Get the term for a given index
    fn term(&self, idx: u64) -> Result<u64> {
        self.mem_storage.term(idx)
    }

    /// Get the first index in storage
    fn first_index(&self) -> Result<u64> {
        self.mem_storage.first_index()
    }

    /// Get the last index in storage
    fn last_index(&self) -> Result<u64> {
        self.mem_storage.last_index()
    }

    /// Create a snapshot at the given index
    fn snapshot(&self, request_index: u64, to: u64) -> Result<Snapshot> {
        self.mem_storage.snapshot(request_index, to)
    }
}
