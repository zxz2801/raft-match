#![allow(clippy::field_reassign_with_default)]

use slog::Drain;
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use protobuf::Message as PbMessage;
use raft::storage::MemStorage;
use raft::{prelude::*, StateRole};

use crate::raft::proposal::Proposal;
use crate::raft::StateMachine;
use slog::{error, o};

fn example_config() -> Config {
    Config {
        election_tick: 10,
        heartbeat_tick: 3,
        ..Default::default()
    }
}

// The message can be used to initialize a raft node or not.
fn is_initial_msg(msg: &Message) -> bool {
    let msg_type = msg.get_msg_type();
    msg_type == MessageType::MsgRequestVote
        || msg_type == MessageType::MsgRequestPreVote
        || (msg_type == MessageType::MsgHeartbeat && msg.commit == 0)
}

pub fn add_all_followers(ids: Vec<u64>, proposals: &Mutex<VecDeque<Proposal>>) {
    for i in ids {
        let mut conf_change = ConfChange::default();
        conf_change.node_id = i;
        conf_change.set_change_type(ConfChangeType::AddNode);
        loop {
            let (proposal, rx) = Proposal::conf_change(&conf_change);
            proposals.lock().unwrap().push_back(proposal);
            break;
            // if rx.recv().unwrap() {
            //     break;
            // }
            // thread::sleep(Duration::from_millis(100));
        }
    }
}

pub struct Node<S: StateMachine> {
    // None if the raft is not initialized.
    raft_group: Option<RawNode<MemStorage>>,
    out_mailbox: Sender<Message>,
    my_mailbox: Receiver<Message>,
    // Key-value pairs after applied. `MemStorage` only contains raft logs,
    // so we need an additional storage engine.
    kv_pairs: HashMap<u16, String>,
    state_machine: S,
    proposals: Arc<Mutex<VecDeque<Proposal>>>,
}

impl<S: StateMachine + Send + Clone + 'static> Node<S> {
    // Create a raft leader only with itself in its configuration.
    fn create_raft_leader(
        id: u64,
        out_mailbox: Sender<Message>,
        my_mailbox: Receiver<Message>,
        proposals: Arc<Mutex<VecDeque<Proposal>>>,
        logger: &slog::Logger,
        state_machine: S,
    ) -> Self {
        let mut cfg = example_config();
        cfg.id = id;
        let logger = logger.new(o!("tag" => format!("peer_{}", id)));
        let mut s = Snapshot::default();
        // Because we don't use the same configuration to initialize every node, so we use
        // a non-zero index to force new followers catch up logs by snapshot first, which will
        // bring all nodes to the same initial state.
        s.mut_metadata().index = 1;
        s.mut_metadata().term = 1;
        s.mut_metadata().mut_conf_state().voters = vec![1];
        let storage = MemStorage::new();
        storage.wl().apply_snapshot(s).unwrap();
        let raft_group = Some(RawNode::new(&cfg, storage, &logger).unwrap());
        Node {
            raft_group,
            out_mailbox,
            my_mailbox,
            kv_pairs: Default::default(),
            proposals,
            state_machine,
        }
    }

    // Create a raft follower.
    fn create_raft_follower(
        id: u64,
        out_mailbox: Sender<Message>,
        my_mailbox: Receiver<Message>,
        proposals: Arc<Mutex<VecDeque<Proposal>>>,
        state_machine: S,
    ) -> Self {
        Node {
            raft_group: None,
            out_mailbox,
            my_mailbox,
            kv_pairs: Default::default(),
            proposals,
            state_machine,
        }
    }

    // Initialize raft for followers.
    fn initialize_raft_from_message(&mut self, msg: &Message, logger: &slog::Logger) {
        if !is_initial_msg(msg) {
            return;
        }
        let mut cfg = example_config();
        cfg.id = msg.to;
        let logger = logger.new(o!("tag" => format!("peer_{}", msg.to)));
        let storage = MemStorage::new();
        self.raft_group = Some(RawNode::new(&cfg, storage, &logger).unwrap());
    }

    // Step a raft message, initialize the raft if need.
    fn step(&mut self, msg: Message, logger: &slog::Logger) {
        if self.raft_group.is_none() {
            if is_initial_msg(&msg) {
                self.initialize_raft_from_message(&msg, logger);
            } else {
                return;
            }
        }
        let raft_group = self.raft_group.as_mut().unwrap();
        let _ = raft_group.step(msg);
    }

    fn run_background_tasks(&mut self, logger: &slog::Logger) {
        let mut t = Instant::now();
        loop {
            thread::sleep(Duration::from_millis(1));
            loop {
                // Step raft messages.
                // 收到其他节点消息 传递到raft系统进行处理 包括心跳 追加日志等等
                match self.my_mailbox.try_recv() {
                    Ok(msg) => self.step(msg, logger),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }

            match self.raft_group {
                Some(ref mut raft_group) => {
                    // 100ms调用一次raft的定时器
                    // 内部触发选主请求 心跳包等
                    if t.elapsed() >= Duration::from_millis(100) {
                        raft_group.tick();
                        t = Instant::now();
                    }

                    // 主节点 投递业务提按到raft系统
                    if raft_group.raft.state == StateRole::Leader {
                        let mut proposals = self.proposals.lock().unwrap();
                        for p in proposals.iter_mut().skip_while(|p| p.proposed > 0) {
                            Self::propose(raft_group, p);
                        }
                    }
                }
                _ => continue,
            }

            // 处理raft事件
            self.on_ready(logger);
        }
    }

    fn handle_out_messages(sender: &Sender<Message>, msgs: Vec<Message>, logger: &slog::Logger) {
        for msg in msgs {
            if (sender.send(msg)).is_err() {
                error!(logger, "send raft message fail, let Raft retry it");
            }
        }
    }

    fn handle_normal_entries(
        rn: &mut RawNode<MemStorage>,
        committed_entries: Vec<Entry>,
        logger: &slog::Logger,
    ) {
        // Apply entries to state machine.
    }

    fn on_ready(&mut self, logger: &slog::Logger) {
        let raft_group = match self.raft_group {
            Some(ref mut r) => r,
            _ => return,
        };
        if !raft_group.has_ready() {
            return;
        }
        let store = raft_group.raft.raft_log.store.clone();

        // Get the `Ready` with `RawNode::ready` interface.
        let mut ready = raft_group.ready();

        let handle_messages = |msgs: Vec<Message>| {
            for msg in msgs {
                if (self.out_mailbox.send(msg)).is_err() {
                    error!(logger, "send raft message fail, let Raft retry it");
                }
            }
        };

        // 处理节点间消息
        if !ready.messages().is_empty() {
            Self::handle_out_messages(&self.out_mailbox, ready.take_messages(), logger);
        }

        // 有新快照 通知storage应用新快照
        if *ready.snapshot() != Snapshot::default() {
            let index = ready.snapshot().get_metadata().index;
            let term = ready.snapshot().get_metadata().term;
            let s = ready.snapshot().clone();
            if let Err(e) = store.wl().apply_snapshot(s) {
                error!(
                    logger,
                    "apply snapshot fail: {:?}, need to retry or panic", e
                );
                return;
            }
            // Apply snapshot to state machine.
            self.state_machine
                .on_snapshot(index, term, ready.snapshot().get_data());
        }

        // 应用状态机 这里需要投递到egging处理
        let mut handle_committed_entries =
            |rn: &mut RawNode<MemStorage>, committed_entries: Vec<Entry>| {
                for entry in committed_entries {
                    if entry.data.is_empty() {
                        // From new elected leaders.
                        continue;
                    }
                    if let EntryType::EntryConfChange = entry.get_entry_type() {
                        // For conf change messages, make them effective.
                        let mut cc = ConfChange::default();
                        cc.merge_from_bytes(&entry.data).unwrap();
                        let cs = rn.apply_conf_change(&cc).unwrap();
                        store.wl().set_conf_state(cs);
                    } else {
                        // 应用状态机
                        self.state_machine.apply(entry.index, entry.data.as_ref());
                    }
                    if rn.raft.state == StateRole::Leader {
                        // The leader should response to the clients, tell them if their proposals
                        // succeeded or not.
                        // 回复消息 需要查找proposals
                        let proposal = self.proposals.lock().unwrap().pop_front().unwrap();
                        match proposal.propose_success.send(true) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(logger, "send proposal event fail: {:?}", e);
                            }
                        }
                    }
                }
            };
        // 处理已经commit的entry
        handle_committed_entries(raft_group, ready.take_committed_entries());

        // Persistent raft logs. It's necessary because in `RawNode::advance` we stabilize
        // raft logs to the latest position.
        // 持久化entries到storage 包括没有commit和commited的
        // raft会根据storage已存储的position来返回entries
        if let Err(e) = store.wl().append(ready.entries()) {
            error!(
                logger,
                "persist raft log fail: {:?}, need to retry or panic", e
            );
            return;
        }

        // 存储raft状态
        if let Some(hs) = ready.hs() {
            // Raft HardState changed, and we need to persist it.
            store.wl().set_hardstate(hs.clone());
        }

        // 通知其他节点 持久化情况
        if !ready.persisted_messages().is_empty() {
            // Send out the persisted messages come from the node.
            handle_messages(ready.take_persisted_messages());
        }

        // Call `RawNode::advance` interface to update position flags in the raft.
        // 存储后推进raft状态
        let mut light_rd = raft_group.advance(ready);
        // Update commit index.
        // 更新已提交index
        if let Some(commit) = light_rd.commit_index() {
            store.wl().mut_hard_state().set_commit(commit);
        }
        // Send out the messages.
        // 发送此次产生的消息
        handle_messages(light_rd.take_messages());
        // Apply all committed entries.
        // 应用状态机
        handle_committed_entries(raft_group, light_rd.take_committed_entries());
        // Advance the apply index.
        raft_group.advance_apply();
    }

    fn propose(raft_group: &mut RawNode<MemStorage>, proposal: &mut Proposal) {
        let last_index1 = raft_group.raft.raft_log.last_index() + 1;
        if let Some((ref key, ref value)) = proposal.normal {
            let data = format!("put {} {}", key, value).into_bytes();
            let _ = raft_group.propose(vec![], data);
        } else if let Some(ref cc) = proposal.conf_change {
            let _ = raft_group.propose_conf_change(vec![], cc.clone());
        } else if let Some(_transferee) = proposal.transfer_leader {
            // TODO: implement transfer leader.
            unimplemented!();
        }

        let last_index2 = raft_group.raft.raft_log.last_index() + 1;
        if last_index2 == last_index1 {
            // Propose failed, don't forget to respond to the client.
            proposal.propose_success.send(false).unwrap();
        } else {
            proposal.proposed = last_index1;
        }
    }
    pub fn start_raft(
        with_leader: bool,
        id: u64,
        rx: Receiver<Message>,
        proposals: Arc<Mutex<VecDeque<Proposal>>>,
        state_machine: S,
    ) -> Receiver<Message> {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = ::slog_term::FullFormat::new(decorator).build().fuse();

        let drain = slog_async::Async::new(drain)
            .chan_size(4096)
            .overflow_strategy(slog_async::OverflowStrategy::Block)
            .build()
            .fuse();
        let logger = slog::Logger::root(drain, o!());

        let (sx, out_mailbox) = mpsc::channel();

        let mut node = match with_leader {
            true => Node::create_raft_leader(id, sx, rx, proposals, &logger, state_machine),
            false => Node::create_raft_follower(id, sx, rx, proposals, state_machine),
        };

        let logger = logger.clone();
        thread::spawn(move || loop {
            node.run_background_tasks(&logger);
        });

        out_mailbox
    }
}
