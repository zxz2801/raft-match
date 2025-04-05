//! Raft node implementation
//! This module contains the core Raft node implementation, including leader and follower nodes,
//! message handling, and state management.

#![allow(clippy::field_reassign_with_default)]

use std::collections::VecDeque;

use slog::Drain;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::{self, Duration, Instant};

use protobuf::Message as PbMessage;
use raft::{prelude::*, StateRole};

use crate::raft::proposal::Proposal;
use crate::raft::StateMachine;
use slog::o;

use super::storage::FileStorage;

// Constants
const TICK_INTERVAL: Duration = Duration::from_millis(100); // Interval for raft tick
const LOGGER_CHANNEL_SIZE: usize = 4096; // Size of logger channel buffer
const SAVE_SNAPSHOT_INTERVAL: Duration = Duration::from_secs(60); // Interval for saving snapshots
/// Default Raft configuration
/// Creates a new Raft configuration with default values
fn default_config(id: u64, applied: u64) -> Config {
    Config {
        id,
        election_tick: 10, // Number of ticks before starting election
        heartbeat_tick: 3, // Number of ticks between heartbeats
        applied,
        ..Default::default()
    }
}

/// Check if the message is used to initialize a raft node
#[allow(unused)]
fn is_initial_msg(msg: &Message) -> bool {
    let msg_type = msg.get_msg_type();
    msg_type == MessageType::MsgRequestVote
        || msg_type == MessageType::MsgRequestPreVote
        || (msg_type == MessageType::MsgHeartbeat && msg.commit == 0)
}

/// Add all followers to the cluster
/// This function adds multiple followers to the Raft cluster through configuration changes
pub async fn add_all_followers(ids: Vec<u64>, proposals: &Sender<Proposal>) {
    for id in ids {
        let mut conf_change = ConfChange::default();
        conf_change.node_id = id;
        conf_change.set_change_type(ConfChangeType::AddNode);
        let (proposal, rx) = Proposal::conf_change(&conf_change);
        let _ = proposals.send(proposal).await;
        match rx.await {
            Ok(ret) => {
                log::info!("Add follower {}, result: {}", id, ret);
            }
            Err(e) => {
                log::error!("Failed to add follower: {:?}", e);
            }
        }
    }
}

/// Raft node implementation
/// This struct represents a Raft node with its associated state and components
pub struct Node<S: StateMachine> {
    raft_group: RawNode<FileStorage>, // The core Raft node implementation
    out_mailbox: Sender<Message>,     // Channel for sending messages to other nodes
    my_mailbox: Receiver<Message>,    // Channel for receiving messages from other nodes
    state_machine: S,                 // The state machine that applies committed entries
    proposals: Receiver<Proposal>,    // Channel for receiving proposals
    proposed: VecDeque<Proposal>,     // Queue of pending proposals
}

impl<S: StateMachine + Send + Clone + 'static> Node<S> {
    /// Create a new raft leader node
    /// Initializes a new Raft node with leader configuration
    fn create_raft_leader(
        id: u64,
        out_mailbox: Sender<Message>,
        my_mailbox: Receiver<Message>,
        proposals: Receiver<Proposal>,
        logger: &slog::Logger,
        state_machine: S,
        base_path: &str,
    ) -> Self {
        let logger = logger.new(o!("tag" => format!("peer_{}", id)));
        let storage = FileStorage::new(base_path, true).unwrap();
        let cfg = default_config(id, storage.commit());
        let raft_group = RawNode::new(&cfg, storage, &logger).unwrap();

        Node {
            raft_group,
            out_mailbox,
            my_mailbox,
            proposals,
            state_machine,
            proposed: VecDeque::new(),
        }
    }

    /// Create a new raft follower node
    /// Initializes a new Raft node with follower configuration
    fn create_raft_follower(
        id: u64,
        out_mailbox: Sender<Message>,
        my_mailbox: Receiver<Message>,
        proposals: Receiver<Proposal>,
        logger: &slog::Logger,
        state_machine: S,
        base_path: &str,
    ) -> Self {
        let logger = logger.new(o!("tag" => format!("peer_{}", id)));
        let storage = FileStorage::new(base_path, false).unwrap();
        let cfg = default_config(id, storage.commit());
        let raft_group = RawNode::new(&cfg, storage, &logger).unwrap();

        Node {
            raft_group,
            out_mailbox,
            my_mailbox,
            proposals,
            state_machine,
            proposed: VecDeque::new(),
        }
    }

    /// Process committed entries
    /// Applies committed entries to the state machine and handles configuration changes
    fn handle_committed_entries(
        raft_group: &mut RawNode<FileStorage>,
        entries: Vec<Entry>,
        state_machine: &mut S,
    ) -> u64 {
        let mut last_index = 0u64;
        for entry in entries {
            if entry.data.is_empty() {
                continue;
            }

            match entry.get_entry_type() {
                EntryType::EntryConfChange => {
                    let mut cc = ConfChange::default();
                    cc.merge_from_bytes(&entry.data).unwrap();
                    let cs = raft_group.apply_conf_change(&cc).unwrap();
                    raft_group.raft.raft_log.store.set_conf_state(cs);
                }
                _ => {
                    state_machine.apply(entry.index, entry.data.as_ref());
                }
            }

            last_index = entry.index;
        }
        last_index
    }

    /// Process raft ready state
    /// Handles the ready state of the Raft node, including message processing,
    /// snapshot handling, and state persistence
    fn on_ready(&mut self) {
        let raft_group = &mut self.raft_group;

        if !raft_group.has_ready() {
            return;
        }

        let mut ready = raft_group.ready();

        // Step 1: Handle messages
        if !ready.messages().is_empty() {
            Self::handle_out_messages(&self.out_mailbox, &ready.take_messages());
        }

        // Step 2: Handle snapshot if any
        if *ready.snapshot() != Snapshot::default() {
            Self::handle_snapshot(raft_group, &ready, &mut self.state_machine);
        }

        // Step 3: Handle committed entries
        let index1 = Self::handle_committed_entries(
            raft_group,
            ready.take_committed_entries(),
            &mut self.state_machine,
        );

        // Step 4: Persist raft state
        Self::persist_raft_state(raft_group, &ready);
        if !ready.persisted_messages().is_empty() {
            Self::handle_out_messages(&self.out_mailbox, &ready.take_persisted_messages());
        }

        // Step 5: Advance raft state
        let mut light_rd = raft_group.advance(ready);
        if let Some(commit) = light_rd.commit_index() {
            Self::update_commit(raft_group, commit);
        }
        Self::handle_out_messages(&self.out_mailbox, light_rd.messages());
        let index2 = Self::handle_committed_entries(
            raft_group,
            light_rd.take_committed_entries(),
            &mut self.state_machine,
        );

        Self::notice_proposed(index1.max(index2), &mut self.proposed);
        raft_group.advance_apply();
    }

    /// Notify proposals about their status
    /// Updates the status of pending proposals based on the last applied index
    fn notice_proposed(last_index: u64, proposed: &mut VecDeque<Proposal>) {
        let mut i = 0;
        while i < proposed.len() {
            if proposed[i].proposed <= last_index {
                let _ = proposed[i].propose_success.take().unwrap().send(true);
                proposed.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Handle raft messages
    /// Sends messages to other nodes in the cluster
    fn handle_out_messages(sender: &Sender<Message>, messages: &[Message]) {
        if !messages.is_empty() {
            for msg in messages {
                if let Err(e) = sender.try_send(msg.clone()) {
                    log::error!("Failed to send raft message {:?}, Raft will retry", e);
                }
            }
        }
    }

    /// Handle snapshot
    /// Applies a snapshot to the state machine and updates the storage
    fn handle_snapshot(
        raft_group: &mut RawNode<FileStorage>,
        ready: &Ready,
        state_machine: &mut S,
    ) {
        let snapshot = ready.snapshot().clone();
        let metadata = snapshot.get_metadata().clone();

        {
            let store = &mut raft_group.raft.raft_log.store;
            if let Err(e) = store.apply_snapshot(&snapshot) {
                log::error!("Failed to apply snapshot: {:?}, need to retry or panic", e);
                return;
            }
        }

        state_machine.on_snapshot(metadata.index, metadata.term, ready.snapshot().get_data());
    }

    /// Handle save snapshot
    /// Creates and saves a snapshot of the current state
    fn handle_save_snapshot(raft_group: &mut RawNode<FileStorage>, state_machine: &mut S) {
        let biz_data = state_machine.snapshot();
        let applied = raft_group.raft.raft_log.applied();
        let store = &mut raft_group.raft.raft_log.store;
        store.save_snapshot(biz_data, applied).unwrap();
        log::info!("Save snapshot at index: {}", applied);
    }

    /// Persist raft state to storage
    /// Saves the current Raft state to persistent storage
    fn persist_raft_state(raft_group: &mut RawNode<FileStorage>, ready: &Ready) {
        let store = &mut raft_group.raft.raft_log.store;

        // Persist entries
        if let Err(e) = store.append_entries(ready.entries()) {
            log::error!(
                "Failed to persist raft log: {:?}, need to retry or panic",
                e
            );
            return;
        }

        // Persist hard state
        if let Some(hs) = ready.hs() {
            store.set_hardstate(hs.clone());
        }
    }

    /// Update commit
    /// Updates the commit index in the storage
    fn update_commit(raft_group: &mut RawNode<FileStorage>, commit: u64) {
        let store = &mut raft_group.raft.raft_log.store;
        store.set_commit(commit);
    }

    /// Run background tasks for the raft node
    /// Main event loop that handles messages, proposals, and periodic tasks
    async fn run_background_tasks(&mut self) {
        let mut last_tick = Instant::now();
        let mut last_save_snapshot = Instant::now();
        let mut last_index_snapshot = 0u64;

        loop {
            let raft_group = &mut self.raft_group;
            tokio::select! {
                Some(outmsg) = self.my_mailbox.recv() => {
                    // Process incoming messages
                    let _ = raft_group.step(outmsg);
                    while let Ok(msg) = self.my_mailbox.try_recv() {
                        let _ = raft_group.step(msg);
                    }
                }
                Some(proposal) = self.proposals.recv() => {
                    // Propose entries if leader
                    Self::propose(raft_group, proposal, &mut self.proposed);
                    while let Ok(proposal) = self.proposals.try_recv() {
                        Self::propose(raft_group, proposal, &mut self.proposed);
                    }
                }
                _ = tokio::time::sleep(time::Duration::from_millis(1)) => {
                }
            }

            // Tick raft
            if last_tick.elapsed() >= TICK_INTERVAL {
                raft_group.tick();
                last_tick = Instant::now();
            }

            // Save snapshot
            if last_save_snapshot.elapsed() >= SAVE_SNAPSHOT_INTERVAL
                && last_index_snapshot < raft_group.raft.raft_log.applied()
            {
                Self::handle_save_snapshot(raft_group, &mut self.state_machine);
                last_save_snapshot = Instant::now();
                last_index_snapshot = raft_group.raft.raft_log.applied();
            }

            // Process ready state
            self.on_ready();
        }
    }

    /// Start a new raft node
    /// Initializes and starts a new Raft node with the specified configuration
    pub fn start_raft(
        with_leader: bool,
        id: u64,
        rx: Receiver<Message>,
        rx_proposals: Receiver<Proposal>,
        state_machine: S,
        base_path: &str,
    ) -> Receiver<Message> {
        // Setup logger
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain)
            .chan_size(LOGGER_CHANNEL_SIZE)
            .overflow_strategy(slog_async::OverflowStrategy::Block)
            .build()
            .fuse();
        let logger = slog::Logger::root(drain, o!());

        let (sx, out_mailbox) = mpsc::channel(1000);

        // Create and start node
        let mut node = if with_leader {
            Node::create_raft_leader(id, sx, rx, rx_proposals, &logger, state_machine, base_path)
        } else {
            Node::create_raft_follower(id, sx, rx, rx_proposals, &logger, state_machine, base_path)
        };

        tokio::spawn(async move {
            node.run_background_tasks().await;
        });

        out_mailbox
    }

    /// Propose a new entry to the raft group
    /// Submits a new proposal to the Raft group if this node is the leader
    fn propose(
        raft_group: &mut RawNode<FileStorage>,
        mut proposal: Proposal,
        proposed: &mut VecDeque<Proposal>,
    ) {
        if raft_group.raft.state != StateRole::Leader {
            return;
        }

        let last_index = raft_group.raft.raft_log.last_index() + 1;

        if let Some(ref data) = proposal.normal {
            let _ = raft_group.propose(vec![], data.clone());
        } else if let Some(ref cc) = proposal.conf_change {
            let _ = raft_group.propose_conf_change(vec![], cc.clone());
        } else if let Some(_transferee) = proposal.transfer_leader {
            // TODO: implement transfer leader.
            unimplemented!();
        }

        let new_last_index = raft_group.raft.raft_log.last_index() + 1;
        if new_last_index == last_index {
            if let Some(sender) = proposal.propose_success.take() {
                let _ = sender.send(false);
            }
        } else {
            proposal.proposed = last_index;
            proposed.push_back(proposal);
        }
    }
}
