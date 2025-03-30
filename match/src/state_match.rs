use crate::engine::matchengine::MatchEngine;
use crate::raft::StateMachine;

#[derive(Default, Clone)]
pub struct StateMatch {
    match_engine: MatchEngine,
}

impl StateMatch {
    pub fn new() -> StateMatch {
        StateMatch {
            match_engine: MatchEngine::new(),
        }
    }
}

impl StateMachine for StateMatch {
    fn apply(&mut self, _index: u64, data: &[u8]) {
        self.match_engine.on_message(data);
    }
    fn snapshot(&self) -> Vec<u8> {
        self.match_engine.snapshot()
    }

    fn on_snapshot(&mut self, _last_index: u64, _last_term: u64, data: &[u8]) {
        self.match_engine.on_snapshot(data);
    }
}
