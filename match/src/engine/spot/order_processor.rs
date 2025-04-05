//! Order Processing Module
//!
//! This module provides functionality for processing orders in the spot market.
//! It handles order placement, cancellation, and symbol management through a unified interface.

use crate::engine::entry::{Order, Symbol, SymbolStatus, Trade};
use crate::engine::spot::SymbolManager;
use serde::{Deserialize, Serialize};

/// Main processor for handling spot market orders
/// Manages symbols and their associated order matching logic
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrderProcessor {
    /// Manager for handling trading symbols
    symbol_manager: SymbolManager,
}

#[allow(unused)]
impl OrderProcessor {
    /// Creates a new order processor with an empty symbol manager
    pub fn new() -> Self {
        Self {
            symbol_manager: SymbolManager::new(),
        }
    }

    /// Places a new order in the market
    ///
    /// # Arguments
    /// * `order` - The order to place
    ///
    /// # Returns
    /// * `Ok(Vec<Trade>)` - List of trades generated from matching this order
    /// * `Err(String)` - Error message if order placement fails
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

    /// Cancels an existing order
    ///
    /// # Arguments
    /// * `symbol_id` - ID of the symbol the order belongs to
    /// * `order_id` - ID of the order to cancel
    ///
    /// # Returns
    /// * `Ok(Some(Order))` - The canceled order if found
    /// * `Ok(None)` - If order was not found
    /// * `Err(String)` - Error message if cancellation fails
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

    /// Adds a new trading symbol
    ///
    /// # Arguments
    /// * `symbol` - The symbol to add
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn add_symbol(&mut self, symbol: Symbol) -> Result<(), String> {
        self.symbol_manager.add_symbol(symbol)
    }

    /// Updates an existing symbol's properties
    ///
    /// # Arguments
    /// * `symbol` - The updated symbol information
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn update_symbol(&mut self, symbol: Symbol) -> Result<(), String> {
        self.symbol_manager.update_symbol(symbol)
    }

    /// Delists (removes) a symbol from trading
    ///
    /// # Arguments
    /// * `symbol` - ID of the symbol to remove
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn del_symbol(&mut self, symbol: &str) -> Result<(), String> {
        self.symbol_manager.delist_symbol(symbol)
    }

    /// Lists all available trading symbols
    ///
    /// # Returns
    /// Vector of references to all symbols
    pub fn list_symbols(&self) -> Vec<&Symbol> {
        self.symbol_manager.list_symbols()
    }
}
