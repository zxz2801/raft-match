pub mod node;
pub mod proposal;
mod segment;
mod storage;

pub trait StateMachine {
    fn apply(&mut self, index: u64, data: &[u8]);
    fn snapshot(&self) -> Vec<u8>;
    fn on_snapshot(&mut self, last_index: u64, last_term: u64, data: &[u8]);
}
