use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub buyer_order_id: String,
    pub seller_order_id: String,
    pub created_at: SystemTime,
}

#[allow(unused)]
impl Trade {
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

    pub fn total_amount(&self) -> Decimal {
        self.price * self.quantity
    }
}
