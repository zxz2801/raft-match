use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub price_precision: i32,
    pub quantity_precision: i32,
    #[serde(skip_serializing, skip_deserializing)]
    pub min_price: Decimal,
    #[serde(skip_serializing, skip_deserializing)]
    pub max_price: Decimal,
    #[serde(skip_serializing, skip_deserializing)]
    pub min_quantity: Decimal,
    #[serde(skip_serializing, skip_deserializing)]
    pub max_quantity: Decimal,
    pub status: SymbolStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolStatus {
    Active,
    Inactive,
    Delisted,
}

#[allow(unused)]
impl Symbol {
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

    pub fn validate_price(&self, price: Decimal) -> bool {
        price >= self.min_price && price <= self.max_price
    }

    pub fn validate_quantity(&self, quantity: Decimal) -> bool {
        quantity >= self.min_quantity && quantity <= self.max_quantity
    }

    fn round_by_precision(value: Decimal, precision: i32) -> Decimal {
        if precision >= 0 {
            value.round_dp(precision as u32)
        } else {
            let factor = Decimal::from(10_i64.pow((-precision) as u32));
            (value * factor).round() / factor
        }
    }

    pub fn round_price(&self, price: Decimal) -> Decimal {
        Self::round_by_precision(price, self.price_precision)
    }

    pub fn round_quantity(&self, quantity: Decimal) -> Decimal {
        Self::round_by_precision(quantity, self.quantity_precision)
    }
}
