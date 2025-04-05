//! Match Logic Module
//!
//! This module implements the core order matching logic for the trading engine.
//! It provides the matching algorithm that pairs buy and sell orders based on
//! price-time priority and order type (market/limit).

pub mod matcher;

pub use matcher::Matcher;
