use crate::engine::data::OrderBook;
use crate::engine::entry::{Order, OrderSide, OrderType, Trade};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct Matcher {
    orderbook: OrderBook,
}

impl Matcher {
    pub fn new(symbol: String) -> Self {
        Self {
            orderbook: OrderBook::new(symbol),
        }
    }

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

    pub fn cancel_order(&mut self, order_id: &str) -> Option<Order> {
        self.orderbook.remove_order(order_id)
    }

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
