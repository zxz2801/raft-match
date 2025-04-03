pub use super::entry::{Order, Symbol};
pub use super::spot::OrderProcessor;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum MatchCmdType {
    #[default]
    PlaceOrder,
    CancelOrder,
    CreateSymbol,
    UpdateSymbol,
    RemoveSymbol,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MatchCmd {
    pub cmd: MatchCmdType,
    pub order: Option<Order>,
    pub symbol: Option<Symbol>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MatchEngine {
    index: u64,
    spot_processor: OrderProcessor,
}

impl MatchEngine {
    pub fn new() -> MatchEngine {
        MatchEngine {
            index: 0,
            spot_processor: OrderProcessor::new(),
        }
    }

    pub fn on_message(&mut self, index: u64, data: &[u8]) {
        log::debug!("on_message: len {}", data.len());
        self.index = index;
        let cmd: Result<MatchCmd, bincode::Error> = bincode::deserialize(data);
        match cmd {
            Ok(cmd) => {
                match cmd.cmd {
                    MatchCmdType::PlaceOrder => {
                        let _ = self.spot_processor.place_order(&cmd.order.unwrap());
                    }
                    MatchCmdType::CancelOrder => {
                        // self.order_book.remove(&cmd.order.unwrap().id);
                    }
                    MatchCmdType::CreateSymbol => {
                        // self.order_book.insert(cmd.symbol.unwrap().id, cmd.symbol.unwrap());
                    }
                    MatchCmdType::UpdateSymbol => {
                        // self.order_book.insert(cmd.symbol.unwrap().id, cmd.symbol.unwrap());
                    }
                    _ => {}
                }
            }
            Err(e) => {
                log::error!("failed to deserialize match cmd: {}", e);
            }
        }
    }

    pub fn on_snapshot(&mut self, data: &[u8]) {
        match bincode::deserialize(data) {
            Ok(match_engine) => *self = match_engine,
            Err(e) => {
                log::error!("failed to deserialize match engine: {}", e);
            }
        }
    }

    pub fn snapshot(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}
