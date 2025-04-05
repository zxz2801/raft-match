//! Spot Market Module
//!
//! This module provides functionality for spot market trading:
//! - `order_processor`: Processes orders and manages their lifecycle
//! - `symbol_manager`: Manages trading symbols and their configurations
//!
//! Together these components handle all spot market operations.

pub mod order_processor;
pub mod symbol_manager;

pub use order_processor::OrderProcessor;
pub use symbol_manager::SymbolManager;
