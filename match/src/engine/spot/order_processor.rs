use crate::engine::entry::{Order, Symbol, SymbolStatus, Trade};
use crate::engine::spot::SymbolManager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrderProcessor {
    symbol_manager: SymbolManager,
}

#[allow(unused)]
impl OrderProcessor {
    pub fn new() -> Self {
        Self {
            symbol_manager: SymbolManager::new(),
        }
    }

    pub fn place_order(&mut self, order: &Order) -> Result<Vec<Trade>, String> {
        // Get symbol info and matcher
        let (symbol_info, matcher) = self
            .symbol_manager
            .get_symbol_and_matcher(&order.symbol)
            .ok_or_else(|| format!("Symbol with id {} does not exist", &order.symbol))?;

        if symbol_info.status != SymbolStatus::Active {
            return Err(format!("Symbol with id {} is not active", &order.symbol));
        }

        // Validate price and quantity
        if !symbol_info.validate_price(order.price) {
            return Err(format!("Invalid price for symbol {}", symbol_info.name));
        }
        if !symbol_info.validate_quantity(order.quantity) {
            return Err(format!("Invalid quantity for symbol {}", symbol_info.name));
        }
        Ok(matcher.place_order(order.clone()))
    }

    pub fn cancel_order(
        &mut self,
        symbol_id: &str,
        order_id: &str,
    ) -> Result<Option<Order>, String> {
        // Get symbol info and matcher
        let (symbol_info, matcher) = self
            .symbol_manager
            .get_symbol_and_matcher(symbol_id)
            .ok_or_else(|| format!("Symbol with id {} does not exist", symbol_id))?;

        if symbol_info.status != SymbolStatus::Active {
            return Err(format!("Symbol with id {} is not active", symbol_id));
        }

        Ok(matcher.cancel_order(order_id))
    }

    pub fn add_symbol(&mut self, symbol: Symbol) -> Result<(), String> {
        self.symbol_manager.add_symbol(symbol)
    }

    pub fn update_symbol(&mut self, symbol: Symbol) -> Result<(), String> {
        self.symbol_manager.update_symbol(symbol)
    }

    pub fn del_symbol(&mut self, symbol: &str) -> Result<(), String> {
        self.symbol_manager.delist_symbol(symbol)
    }

    pub fn list_symbols(&self) -> Vec<&Symbol> {
        self.symbol_manager.list_symbols()
    }
}
