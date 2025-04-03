//! Raft proposal implementation
//! This module defines the Proposal struct and its associated functionality for
//! handling different types of Raft proposals.

#![allow(clippy::field_reassign_with_default)]

use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;
use tokio::sync::oneshot::Sender;

use raft::prelude::*;

/// Represents a proposal that can be submitted to the Raft cluster
/// A proposal can be one of three types: normal entry, configuration change, or leader transfer
pub struct Proposal {
    /// Normal proposal data (key-value pair where key is u16 and value is string)
    pub normal: Option<Vec<u8>>,
    /// Configuration change proposal
    pub conf_change: Option<ConfChange>,
    /// Leader transfer proposal
    pub transfer_leader: Option<u64>,
    /// The index at which this proposal was proposed (0 if not yet proposed)
    pub proposed: u64,
    /// Channel for notifying the proposer about the success/failure of the proposal
    pub propose_success: Option<Sender<bool>>,
}

impl Proposal {
    /// Create a new configuration change proposal
    /// Returns the proposal and a receiver for the proposal result
    pub fn conf_change(cc: &ConfChange) -> (Self, Receiver<bool>) {
        let (tx, rx) = oneshot::channel();
        let proposal = Proposal {
            normal: None,
            conf_change: Some(cc.clone()),
            transfer_leader: None,
            proposed: 0,
            propose_success: Some(tx),
        };
        (proposal, rx)
    }

    /// Create a new normal proposal
    /// Returns the proposal and a receiver for the proposal result
    pub fn normal(data: Vec<u8>) -> (Self, Receiver<bool>) {
        let (tx, rx) = oneshot::channel();
        let proposal = Proposal {
            normal: Some(data),
            conf_change: None,
            transfer_leader: None,
            proposed: 0,
            propose_success: Some(tx),
        };
        (proposal, rx)
    }
}
