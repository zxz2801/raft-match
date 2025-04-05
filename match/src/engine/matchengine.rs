//! Match Engine Module
//!
//! This module implements the core matching engine functionality for processing orders and symbols.
//! It handles order placement, cancellation, and symbol management through a state machine interface.

pub use super::entry::{Order, Symbol};
pub use super::spot::OrderProcessor;

use serde::{Deserialize, Serialize};

/// Represents the different types of commands that can be processed by the match engine
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum MatchCmdType {
    /// Place a new order in the order book
    #[default]
    PlaceOrder,
    /// Cancel an existing order
    CancelOrder,
    /// Create a new trading symbol
    CreateSymbol,
    /// Update an existing symbol's properties
    UpdateSymbol,
    /// Remove a symbol from trading
    RemoveSymbol,
}

/// Command structure for interacting with the match engine
/// Contains the command type and associated data (order or symbol)
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MatchCmd {
    /// The type of command to execute
    pub cmd: MatchCmdType,
    /// Optional order data for order-related commands
    pub order: Option<Order>,
    /// Optional symbol data for symbol-related commands
    pub symbol: Option<Symbol>,
}

/// The main match engine implementation
/// Maintains the current state of the order book and processes trading commands
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MatchEngine {
    /// Current index/version of the engine state
    index: u64,
    /// Processor for handling spot market orders
    spot_processor: OrderProcessor,
}

impl MatchEngine {
    /// Creates a new instance of the match engine
    /// Initializes with default state and a new order processor
    pub fn new() -> MatchEngine {
        MatchEngine {
            index: 0,
            spot_processor: OrderProcessor::new(),
        }
    }

    /// Processes an incoming message/command
    ///
    /// # Arguments
    /// * `index` - The new index/version number for this state update
    /// * `data` - Serialized command data to process
    pub fn on_message(&mut self, index: u64, data: &[u8]) {
        log::debug!("on_message: len {}", data.len());
        self.index = index;
        let cmd: Result<MatchCmd, bincode::Error> = bincode::deserialize(data);
        match cmd {
            Ok(cmd) => match cmd.cmd {
                MatchCmdType::PlaceOrder => {
                    let _ = self.spot_processor.place_order(&cmd.order.unwrap());
                }
                MatchCmdType::CancelOrder => {
                    let symbol = cmd.order.as_ref().unwrap().symbol.clone();
                    let order_id = cmd.order.as_ref().unwrap().id.clone();
                    let _ = self.spot_processor.cancel_order(&symbol, &order_id);
                }
                MatchCmdType::CreateSymbol => {
                    let _ = self.spot_processor.add_symbol(cmd.symbol.unwrap());
                }
                MatchCmdType::RemoveSymbol => {
                    let symbol = cmd.symbol.as_ref().unwrap().name.clone();
                    let _ = self.spot_processor.del_symbol(&symbol);
                }
                _ => {}
            },
            Err(e) => {
                log::error!("failed to deserialize match cmd: {}", e);
            }
        }
    }

    /// Restores engine state from a snapshot
    ///
    /// # Arguments
    /// * `data` - Serialized engine state data
    pub fn on_snapshot(&mut self, data: &[u8]) {
        match bincode::deserialize(data) {
            Ok(match_engine) => *self = match_engine,
            Err(e) => {
                log::error!("failed to deserialize match engine: {}", e);
            }
        }
    }

    /// Creates a snapshot of the current engine state
    ///
    /// # Returns
    /// Serialized engine state as a byte vector
    pub fn snapshot(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}
