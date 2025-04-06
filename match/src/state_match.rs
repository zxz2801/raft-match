//! State machine implementation for the match service
//!
//! This module implements the Raft state machine interface for the match engine.

use crate::engine::matchengine::MatchEngine;
use crate::raft::StateMachine;

/// State machine that wraps the match engine
///
/// This struct implements the Raft state machine interface and delegates
/// operations to the underlying match engine.
#[derive(Default, Clone)]
pub struct StateMatch {
    /// The match engine instance
    match_engine: MatchEngine,
}

impl StateMatch {
    /// Creates a new StateMatch instance
    pub fn new() -> StateMatch {
        StateMatch {
            match_engine: MatchEngine::new(),
        }
    }
}

impl StateMachine for StateMatch {
    /// Applies a log entry to the state machine
    ///
    /// # Arguments
    ///
    /// * `index` - The log index of the entry
    /// * `data` - The data to apply
    fn apply(&mut self, index: u64, data: &[u8]) {
        self.match_engine.on_message(index, data);
    }

    /// Creates a snapshot of the current state
    ///
    /// # Returns
    ///
    /// Returns a byte vector containing the serialized state
    fn snapshot(&self) -> Vec<u8> {
        self.match_engine.snapshot()
    }

    /// Restores state from a snapshot
    ///
    /// # Arguments
    ///
    /// * `_last_index` - The last applied log index
    /// * `_last_term` - The last applied log term
    /// * `data` - The snapshot data to restore from
    fn on_snapshot(&mut self, _last_index: u64, _last_term: u64, data: &[u8]) {
        if !data.is_empty() {
            self.match_engine.on_snapshot(data);
        }
    }
}
