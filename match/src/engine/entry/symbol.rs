//! Symbol Types and Structures
//!
//! This module defines the trading symbol structure and related functionality.
//! It includes validation and precision handling for prices and quantities.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a trading symbol in the system
/// Contains configuration for price and quantity precision, limits, and validation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Symbol {
    /// Name of the trading symbol (e.g., "BTC/USDT")
    pub name: String,
    /// Base currency of the trading pair (e.g., "BTC")
    pub base_currency: String,
    /// Quote currency of the trading pair (e.g., "USDT")
    pub quote_currency: String,
    /// Number of decimal places for price
    pub price_precision: i32,
    /// Number of decimal places for quantity
    pub quantity_precision: i32,
    /// Minimum allowed price
    pub min_price: Decimal,
    /// Maximum allowed price
    pub max_price: Decimal,
    /// Minimum allowed quantity
    pub min_quantity: Decimal,
    /// Maximum allowed quantity
    pub max_quantity: Decimal,
    /// Current status of the symbol
    pub status: SymbolStatus,
    /// Timestamp when the symbol was created
    pub created_at: u64,
    /// Timestamp when the symbol was last updated
    pub updated_at: u64,
}

/// Represents the current status of a trading symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SymbolStatus {
    /// Symbol is active and accepting orders
    #[default]
    Active,
    /// Symbol is temporarily inactive
    Inactive,
    /// Symbol has been permanently removed
    Delisted,
}

#[allow(unused)]
impl Symbol {
    /// Creates a new trading symbol with the specified parameters
    /// 
    /// # Arguments
    /// * `name` - Name of the trading pair
    /// * `base_currency` - Base currency code
    /// * `quote_currency` - Quote currency code
    /// * `price_precision` - Number of decimal places for price
    /// * `quantity_precision` - Number of decimal places for quantity
    /// * `min_price` - Minimum allowed price
    /// * `max_price` - Maximum allowed price
    /// * `min_quantity` - Minimum allowed quantity
    /// * `max_quantity` - Maximum allowed quantity
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        base_currency: String,
        quote_currency: String,
        price_precision: i32,
        quantity_precision: i32,
        min_price: Decimal,
        max_price: Decimal,
        min_quantity: Decimal,
        max_quantity: Decimal,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            name,
            base_currency,
            quote_currency,
            price_precision,
            quantity_precision,
            min_price,
            max_price,
            min_quantity,
            max_quantity,
            status: SymbolStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    /// Validates if a price is within the allowed range
    /// 
    /// # Arguments
    /// * `price` - Price to validate
    /// 
    /// # Returns
    /// True if the price is within min_price and max_price
    pub fn validate_price(&self, price: Decimal) -> bool {
        price >= self.min_price && price <= self.max_price
    }

    /// Validates if a quantity is within the allowed range
    /// 
    /// # Arguments
    /// * `quantity` - Quantity to validate
    /// 
    /// # Returns
    /// True if the quantity is within min_quantity and max_quantity
    pub fn validate_quantity(&self, quantity: Decimal) -> bool {
        quantity >= self.min_quantity && quantity <= self.max_quantity
    }

    /// Rounds a value to the specified precision
    /// 
    /// # Arguments
    /// * `value` - Value to round
    /// * `precision` - Number of decimal places (can be negative for rounding to powers of 10)
    /// 
    /// # Returns
    /// Rounded value
    fn round_by_precision(value: Decimal, precision: i32) -> Decimal {
        if precision >= 0 {
            value.round_dp(precision as u32)
        } else {
            let factor = Decimal::from(10_i64.pow((-precision) as u32));
            (value * factor).round() / factor
        }
    }

    /// Rounds a price according to the symbol's price precision
    /// 
    /// # Arguments
    /// * `price` - Price to round
    /// 
    /// # Returns
    /// Rounded price
    pub fn round_price(&self, price: Decimal) -> Decimal {
        Self::round_by_precision(price, self.price_precision)
    }

    /// Rounds a quantity according to the symbol's quantity precision
    /// 
    /// # Arguments
    /// * `quantity` - Quantity to round
    /// 
    /// # Returns
    /// Rounded quantity
    pub fn round_quantity(&self, quantity: Decimal) -> Decimal {
        Self::round_by_precision(quantity, self.quantity_precision)
    }
}
