use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

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
    pub price: Decimal,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

impl Order {
    pub fn new(
        id: String,
        symbol: String,
        order_type: OrderType,
        side: OrderSide,
        price: Decimal,
        quantity: Decimal,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            symbol,
            order_type,
            side,
            price,
            quantity,
            filled_quantity: dec!(0),
            status: OrderStatus::New,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn default() -> Self {
        let now = SystemTime::now();
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
        self.updated_at = SystemTime::now();
    }
}
