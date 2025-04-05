//! Entry Types Module
//!
//! This module contains the core data types and structures used throughout the matching engine:
//! - `order`: Order types and related functionality
//! - `symbol`: Trading symbol definitions and validation
//! - `trade`: Trade execution records and calculations
//!
//! These types form the foundation of the matching engine's data model.

pub mod order;
pub mod symbol;
pub mod trade;

pub use order::{Order, OrderSide, OrderType};
pub use symbol::{Symbol, SymbolStatus};
pub use trade::Trade;
