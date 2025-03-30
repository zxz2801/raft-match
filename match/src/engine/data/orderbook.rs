use crate::engine::entry::{Order, OrderSide};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: BTreeMap<Decimal, Vec<Order>>, // price -> orders
    pub asks: BTreeMap<Decimal, Vec<Order>>, // price -> orders
    pub orders_by_id: HashMap<String, Order>,
}

impl OrderBook {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders_by_id: HashMap::new(),
        }
    }

    pub fn add_order(&mut self, order: Order) {
        let orders = match order.side {
            OrderSide::Buy => self.bids.entry(order.price).or_insert_with(Vec::new),
            OrderSide::Sell => self.asks.entry(order.price).or_insert_with(Vec::new),
        };
        orders.push(order.clone());
        self.orders_by_id.insert(order.id.clone(), order);
    }

    pub fn remove_order(&mut self, order_id: &str) -> Option<Order> {
        if let Some(order) = self.orders_by_id.remove(order_id) {
            let orders = match order.side {
                OrderSide::Buy => self.bids.get_mut(&order.price),
                OrderSide::Sell => self.asks.get_mut(&order.price),
            };

            if let Some(orders) = orders {
                orders.retain(|o| o.id != order_id);
                if orders.is_empty() {
                    match order.side {
                        OrderSide::Buy => self.bids.remove(&order.price),
                        OrderSide::Sell => self.asks.remove(&order.price),
                    };
                }
            }
            Some(order)
        } else {
            None
        }
    }

    pub fn get_order(&self, order_id: &str) -> Option<&Order> {
        self.orders_by_id.get(order_id)
    }

    pub fn get_best_bid(&self) -> Option<Decimal> {
        self.bids.keys().next_back().map(|&k| (k))
    }

    pub fn get_best_ask(&self) -> Option<Decimal> {
        self.asks.keys().next().map(|&k| (k))
    }

    pub fn get_spread(&self) -> Option<Decimal> {
        match (self.get_best_ask(), self.get_best_bid()) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }
}
