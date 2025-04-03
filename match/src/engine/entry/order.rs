use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OrderType {
    #[default]
    Market,
    Limit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OrderSide {
    #[default]
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OrderStatus {
    #[default]
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub symbol: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    #[serde(skip_serializing, skip_deserializing)]
    pub price: Decimal,
    #[serde(skip_serializing, skip_deserializing)]
    pub quantity: Decimal,
    #[serde(skip_serializing, skip_deserializing)]
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub price_str: String,
    pub quantity_str: String,
    pub filled_quantity_str: String,
}

#[allow(unused)]
impl Order {
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
            price_str: price.clone(),
            quantity_str: quantity.clone(),
            filled_quantity_str: dec!(0).to_string(),
            status: OrderStatus::New,
            created_at: now,
            updated_at: now,
            price: Decimal::from_str(&price).unwrap(),
            quantity: Decimal::from_str(&quantity).unwrap(),
            filled_quantity: dec!(0),
        }
    }

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
            price_str: dec!(0).to_string(),
            quantity_str: dec!(0).to_string(),
            filled_quantity_str: dec!(0).to_string(),
        }
    }

    pub fn remaining_quantity(&self) -> Decimal {
        self.quantity - self.filled_quantity
    }

    pub fn is_filled(&self) -> bool {
        self.filled_quantity >= self.quantity
    }

    pub fn is_cancelable(&self) -> bool {
        matches!(self.status, OrderStatus::New | OrderStatus::PartiallyFilled)
    }

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
