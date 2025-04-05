//! Order Matching Logic Module
//!
//! This module implements the core order matching logic for the trading engine.
//! It handles matching of market and limit orders according to price-time priority.

use crate::engine::data::OrderBook;
use crate::engine::entry::{Order, OrderSide, OrderType, Trade};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Core order matching engine for a single trading symbol
/// Maintains an order book and implements matching logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Matcher {
    /// Order book containing all active orders
    orderbook: OrderBook,
}

impl Matcher {
    /// Creates a new matcher for a specific trading symbol
    /// 
    /// # Arguments
    /// * `symbol` - Name of the trading symbol
    pub fn new(symbol: String) -> Self {
        Self {
            orderbook: OrderBook::new(symbol),
        }
    }

    /// Places a new order and attempts to match it with existing orders
    /// 
    /// # Arguments
    /// * `order` - The order to place and match
    /// 
    /// # Returns
    /// Vector of trades generated from matching this order
    pub fn place_order(&mut self, mut order: Order) -> Vec<Trade> {
        let mut trades = Vec::new();

        match order.order_type {
            OrderType::Market => {
                trades.extend(self.match_market_order(&mut order));
            }
            OrderType::Limit => {
                trades.extend(self.match_limit_order(&mut order));
            }
        }

        if !order.is_filled() {
            self.orderbook.add_order(order);
        }

        trades
    }

    /// Cancels an existing order
    /// 
    /// # Arguments
    /// * `order_id` - ID of the order to cancel
    /// 
    /// # Returns
    /// The canceled order if found, None otherwise
    pub fn cancel_order(&mut self, order_id: &str) -> Option<Order> {
        self.orderbook.remove_order(order_id)
    }

    /// Matches a market order against the order book
    /// Market orders are executed at the best available price
    /// 
    /// # Arguments
    /// * `order` - The market order to match
    /// 
    /// # Returns
    /// Vector of trades generated from matching this order
    fn match_market_order(&mut self, order: &mut Order) -> Vec<Trade> {
        let mut trades = Vec::new();

        while !order.is_filled() {
            let best_price = match order.side {
                OrderSide::Buy => self.orderbook.get_best_ask(),
                OrderSide::Sell => self.orderbook.get_best_bid(),
            };

            if best_price.is_none() {
                break;
            }

            let price = best_price.unwrap();
            let orders = match order.side {
                OrderSide::Buy => self.orderbook.asks.get_mut(&price),
                OrderSide::Sell => self.orderbook.bids.get_mut(&price),
            };

            if let Some(orders) = orders {
                if let Some(matching_order) = orders.first_mut() {
                    let trade_quantity = order
                        .remaining_quantity()
                        .min(matching_order.remaining_quantity());
                    let trade = Trade::new(
                        Uuid::new_v4().to_string(),
                        order.symbol.clone(),
                        price,
                        trade_quantity,
                        if order.side == OrderSide::Buy {
                            order.id.clone()
                        } else {
                            matching_order.id.clone()
                        },
                        if order.side == OrderSide::Buy {
                            matching_order.id.clone()
                        } else {
                            order.id.clone()
                        },
                    );

                    order.filled_quantity += trade_quantity;
                    matching_order.filled_quantity += trade_quantity;
                    order.update_status();
                    matching_order.update_status();
                    trades.push(trade);

                    if matching_order.is_filled() {
                        orders.remove(0);
                        if orders.is_empty() {
                            match order.side {
                                OrderSide::Buy => self.orderbook.asks.remove(&price),
                                OrderSide::Sell => self.orderbook.bids.remove(&price),
                            };
                        }
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        trades
    }

    /// Matches a limit order against the order book
    /// Limit orders are only executed at their specified price or better
    /// 
    /// # Arguments
    /// * `order` - The limit order to match
    /// 
    /// # Returns
    /// Vector of trades generated from matching this order
    fn match_limit_order(&mut self, order: &mut Order) -> Vec<Trade> {
        let mut trades = Vec::new();

        while !order.is_filled() {
            let can_match = match order.side {
                OrderSide::Buy => {
                    if let Some(best_ask) = self.orderbook.get_best_ask() {
                        order.price >= best_ask
                    } else {
                        false
                    }
                }
                OrderSide::Sell => {
                    if let Some(best_bid) = self.orderbook.get_best_bid() {
                        order.price <= best_bid
                    } else {
                        false
                    }
                }
            };

            if !can_match {
                break;
            }

            trades.extend(self.match_market_order(order));
        }

        trades
    }
}
