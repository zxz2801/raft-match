#![allow(clippy::field_reassign_with_default)]

use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;
use tokio::sync::oneshot::Sender;

use raft::prelude::*;

pub struct Proposal {
    pub normal: Option<Vec<u8>>, // key is an u16 integer, and value is a string.
    pub conf_change: Option<ConfChange>, // conf change.
    pub transfer_leader: Option<u64>,
    // If it's proposed, it will be set to the index of the entry.
    pub proposed: u64,
    pub propose_success: Option<Sender<bool>>,
}

impl Proposal {
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
