//! Order Types and Structures
//!
//! This module defines the core order-related types and structures used in the matching engine.
//! It includes order types, sides, statuses, and the main Order structure with its operations.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents the type of an order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OrderType {
    /// Market order - executed at the best available price
    #[default]
    Market,
    /// Limit order - executed at a specific price or better
    Limit,
}

/// Represents the side of an order (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OrderSide {
    /// Buy order - seeking to purchase an asset
    #[default]
    Buy,
    /// Sell order - seeking to sell an asset
    Sell,
}

/// Represents the current status of an order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OrderStatus {
    /// New order that hasn't been processed yet
    #[default]
    New,
    /// Order has been partially filled
    PartiallyFilled,
    /// Order has been completely filled
    Filled,
    /// Order has been canceled
    Canceled,
    /// Order was rejected
    Rejected,
}

/// Represents a trading order in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Unique identifier for the order
    pub id: String,
    /// Trading symbol for the order
    pub symbol: String,
    /// Type of the order (Market or Limit)
    pub order_type: OrderType,
    /// Side of the order (Buy or Sell)
    pub side: OrderSide,
    /// Price of the order (for Limit orders)
    pub price: Decimal,
    /// Total quantity of the order
    pub quantity: Decimal,
    /// Quantity that has been filled
    pub filled_quantity: Decimal,
    /// Current status of the order
    pub status: OrderStatus,
    /// Timestamp when the order was created
    pub created_at: u64,
    /// Timestamp when the order was last updated
    pub updated_at: u64,
}

#[allow(unused)]
impl Order {
    /// Creates a new order with the specified parameters
    /// 
    /// # Arguments
    /// * `id` - Unique identifier for the order
    /// * `symbol` - Trading symbol
    /// * `order_type` - Type of order (Market/Limit)
    /// * `side` - Side of order (Buy/Sell)
    /// * `price` - Price as a string (will be parsed to Decimal)
    /// * `quantity` - Quantity as a string (will be parsed to Decimal)
    pub fn new(
        id: String,
        symbol: String,
        order_type: OrderType,
        side: OrderSide,
        price: String,
        quantity: String,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            id,
            symbol,
            order_type,
            side,
            status: OrderStatus::New,
            created_at: now,
            updated_at: now,
            price: Decimal::from_str(&price).unwrap(),
            quantity: Decimal::from_str(&quantity).unwrap(),
            filled_quantity: dec!(0),
        }
    }

    /// Creates a new default order with empty values
    pub fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            id: String::new(),
            symbol: String::new(),
            order_type: OrderType::default(),
            side: OrderSide::default(),
            price: dec!(0),
            quantity: dec!(0),
            filled_quantity: dec!(0),
            status: OrderStatus::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Calculates the remaining quantity to be filled
    /// 
    /// # Returns
    /// The difference between total quantity and filled quantity
    pub fn remaining_quantity(&self) -> Decimal {
        self.quantity - self.filled_quantity
    }

    /// Checks if the order has been completely filled
    /// 
    /// # Returns
    /// True if filled quantity is greater than or equal to total quantity
    pub fn is_filled(&self) -> bool {
        self.filled_quantity >= self.quantity
    }

    /// Checks if the order can be canceled
    /// 
    /// # Returns
    /// True if the order is in New or PartiallyFilled state
    pub fn is_cancelable(&self) -> bool {
        matches!(self.status, OrderStatus::New | OrderStatus::PartiallyFilled)
    }

    /// Updates the order status based on its current state
    /// Also updates the updated_at timestamp
    pub fn update_status(&mut self) {
        if self.is_filled() {
            self.status = OrderStatus::Filled;
        } else if self.filled_quantity > dec!(0) {
            self.status = OrderStatus::PartiallyFilled;
        }
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}
