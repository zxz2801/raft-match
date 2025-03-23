use slog::Drain;
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{str, thread};

use protobuf::Message as PbMessage;
use raft::storage::MemStorage;
use raft::{prelude::*, StateRole};
use regex::Regex;

use slog::{error, info, o};

pub mod node;
pub mod proposal;
mod segment;
mod storage;

pub trait StateMachine {
    fn apply(&mut self, index: u64, data: &[u8]);
    fn snapshot(&self) -> Vec<u8>;
    fn on_snapshot(&mut self, last_index: u64, last_term: u64, data: &[u8]);
}
