//! Raft implementation
//! This module provides a Raft consensus implementation with file-based storage.

pub mod node; // Raft node implementation
pub mod proposal; // Proposal handling
mod segment; // File segment implementation
mod storage; // Storage implementation

/// Trait for implementing a state machine that can be managed by Raft
/// The state machine is responsible for applying committed entries and handling snapshots
pub trait StateMachine {
    /// Apply a committed entry to the state machine
    fn apply(&mut self, index: u64, data: &[u8]);

    /// Create a snapshot of the current state machine state
    fn snapshot(&self) -> Vec<u8>;

    /// Restore the state machine from a snapshot
    fn on_snapshot(&mut self, last_index: u64, last_term: u64, data: &[u8]);
}
