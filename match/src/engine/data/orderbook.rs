//! Order Book Implementation
//!
//! This module provides the core order book data structure and operations for managing
//! buy and sell orders in a trading system.

use crate::engine::entry::{Order, OrderSide};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Represents an order book for a specific trading symbol
/// Maintains separate collections for buy (bids) and sell (asks) orders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    /// The trading symbol this order book represents
    pub symbol: String,
    /// Buy orders organized by price level (highest to lowest)
    pub bids: BTreeMap<Decimal, Vec<Order>>,
    /// Sell orders organized by price level (lowest to highest)
    pub asks: BTreeMap<Decimal, Vec<Order>>,
    /// Quick lookup map for orders by their ID
    pub orders_by_id: HashMap<String, Order>,
}

#[allow(unused)]
impl OrderBook {
    /// Creates a new empty order book for the specified symbol
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders_by_id: HashMap::new(),
        }
    }

    /// Adds a new order to the order book
    /// 
    /// # Arguments
    /// * `order` - The order to add to the book
    pub fn add_order(&mut self, order: Order) {
        let orders = match order.side {
            OrderSide::Buy => self.bids.entry(order.price).or_default(),
            OrderSide::Sell => self.asks.entry(order.price).or_default(),
        };
        orders.push(order.clone());
        self.orders_by_id.insert(order.id.clone(), order);
    }

    /// Removes an order from the order book by its ID
    /// 
    /// # Arguments
    /// * `order_id` - The ID of the order to remove
    /// 
    /// # Returns
    /// The removed order if found, None otherwise
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

    /// Retrieves an order by its ID
    /// 
    /// # Arguments
    /// * `order_id` - The ID of the order to retrieve
    /// 
    /// # Returns
    /// A reference to the order if found, None otherwise
    pub fn get_order(&self, order_id: &str) -> Option<&Order> {
        self.orders_by_id.get(order_id)
    }

    /// Gets the highest bid price in the order book
    /// 
    /// # Returns
    /// The best bid price if available, None if there are no bids
    pub fn get_best_bid(&self) -> Option<Decimal> {
        self.bids.keys().next_back().copied()
    }

    /// Gets the lowest ask price in the order book
    /// 
    /// # Returns
    /// The best ask price if available, None if there are no asks
    pub fn get_best_ask(&self) -> Option<Decimal> {
        self.asks.keys().next().copied()
    }

    /// Calculates the current spread between best ask and best bid
    /// 
    /// # Returns
    /// The spread (ask - bid) if both sides have orders, None otherwise
    pub fn get_spread(&self) -> Option<Decimal> {
        match (self.get_best_ask(), self.get_best_bid()) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }
}
