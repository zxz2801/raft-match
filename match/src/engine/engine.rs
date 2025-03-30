pub use super::data::OrderBook;
pub use super::entry::{Order, OrderSide, OrderStatus, OrderType, Symbol, SymbolStatus, Trade};
pub use super::matchlogic::Matcher;
pub use super::spot::OrderProcessor;
pub use super::spot::SymbolManager;

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
    spot_processor: OrderProcessor,
}

impl MatchEngine {
    pub fn new() -> MatchEngine {
        MatchEngine {
            spot_processor: OrderProcessor::new(),
        }
    }

    pub fn on_message(&mut self, data: &[u8]) {
        let cmd: MatchCmd = bincode::deserialize(data).unwrap();
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

    pub fn on_snapshot(&mut self, data: &[u8]) {
        match bincode::deserialize(data) {
            Ok(order_book) => *self = order_book,
            Err(e) => {
                log::error!("failed to deserialize order book: {}", e);
            }
        }
    }

    pub fn snapshot(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}
