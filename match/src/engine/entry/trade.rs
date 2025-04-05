//! Trade Types and Structures
//!
//! This module defines the trade structure and related functionality.
//! A trade represents a completed transaction between a buyer and seller.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Represents a completed trade in the system
/// Contains information about the matched orders and trade details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    /// Unique identifier for the trade
    pub id: String,
    /// Trading symbol for the trade
    pub symbol: String,
    /// Price at which the trade was executed
    pub price: Decimal,
    /// Quantity of the trade
    pub quantity: Decimal,
    /// ID of the buyer's order
    pub buyer_order_id: String,
    /// ID of the seller's order
    pub seller_order_id: String,
    /// Timestamp when the trade was created
    pub created_at: SystemTime,
}

#[allow(unused)]
impl Trade {
    /// Creates a new trade with the specified parameters
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the trade
    /// * `symbol` - Trading symbol
    /// * `price` - Execution price
    /// * `quantity` - Trade quantity
    /// * `buyer_order_id` - ID of the buyer's order
    /// * `seller_order_id` - ID of the seller's order
    pub fn new(
        id: String,
        symbol: String,
        price: Decimal,
        quantity: Decimal,
        buyer_order_id: String,
        seller_order_id: String,
    ) -> Self {
        Self {
            id,
            symbol,
            price,
            quantity,
            buyer_order_id,
            seller_order_id,
            created_at: SystemTime::now(),
        }
    }

    /// Calculates the total amount of the trade
    ///
    /// # Returns
    /// The product of price and quantity
    pub fn total_amount(&self) -> Decimal {
        self.price * self.quantity
    }
}
