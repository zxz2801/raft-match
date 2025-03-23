/*
TODO!
1. raft 收到entity后 append到storage segment中
2. node 生产&或者 读取到snapshot后 存储到storage
3. 存储snapshort删除index前的segment
4. memstorage 存储最近的未commit的entity
5. 维护segment index范围 重传时需要用到
*/

use crate::raft::StateMachine;
use raft::eraftpb::Entry;
use raft::eraftpb::Snapshot;
use raft::{storage::MemStorage, GetEntriesContext, RaftState, Result, Storage};

pub struct FileStorage {
    mem_storage: MemStorage,
    state_machine: Box<dyn StateMachine>,
}

impl Storage for FileStorage {
    /// Implements the Storage trait.
    fn initial_state(&self) -> Result<RaftState> {
        self.mem_storage.initial_state()
    }

    /// Implements the Storage trait.
    fn entries(
        &self,
        low: u64,
        high: u64,
        max_size: impl Into<Option<u64>>,
        context: GetEntriesContext,
    ) -> Result<Vec<Entry>> {
        // 持久化到segments
        self.mem_storage.entries(low, high, max_size, context)
    }

    /// Implements the Storage trait.
    fn term(&self, idx: u64) -> Result<u64> {
        self.mem_storage.term(idx)
    }

    /// Implements the Storage trait.
    fn first_index(&self) -> Result<u64> {
        self.mem_storage.first_index()
    }

    /// Implements the Storage trait.
    fn last_index(&self) -> Result<u64> {
        self.mem_storage.last_index()
    }

    /// Implements the Storage trait.
    fn snapshot(&self, request_index: u64, _to: u64) -> Result<Snapshot> {
        // state_machine snapshot
        // 持久化到文件
        // 删除index前的segment
        self.mem_storage.snapshot(request_index, _to)
    }
}
