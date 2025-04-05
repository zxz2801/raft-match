//! Symbol Management Module
//!
//! This module provides functionality for managing trading symbols and their associated matchers.
//! It handles symbol lifecycle operations including creation, updates, deactivation, and delisting.

use crate::engine::entry::{Symbol, SymbolStatus};
use crate::engine::matchlogic::Matcher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manager for handling trading symbols and their associated order matchers
/// Maintains the lifecycle and state of all trading symbols in the system
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SymbolManager {
    /// Map of symbol names to their configurations
    symbols: HashMap<String, Symbol>,
    /// Map of symbol names to their order matchers
    matchers: HashMap<String, Matcher>,
}

#[allow(unused)]
impl SymbolManager {
    /// Creates a new empty symbol manager
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            matchers: HashMap::new(),
        }
    }

    /// Adds a new trading symbol to the system
    ///
    /// # Arguments
    /// * `symbol` - The symbol configuration to add
    ///
    /// # Returns
    /// * `Ok(())` - If symbol was added successfully
    /// * `Err(String)` - If symbol already exists
    pub fn add_symbol(&mut self, symbol: Symbol) -> Result<(), String> {
        if self.symbols.contains_key(&symbol.name) {
            return Err(format!("Symbol {} already exists", symbol.name));
        }

        self.symbols.insert(symbol.name.clone(), symbol.clone());
        self.matchers
            .insert(symbol.name.clone(), Matcher::new(symbol.name.clone()));
        Ok(())
    }

    /// Updates an existing symbol's configuration
    ///
    /// # Arguments
    /// * `symbol` - The updated symbol configuration
    ///
    /// # Returns
    /// * `Ok(())` - If symbol was updated successfully
    /// * `Err(String)` - If symbol does not exist
    pub fn update_symbol(&mut self, symbol: Symbol) -> Result<(), String> {
        if !self.symbols.contains_key(&symbol.name) {
            return Err(format!("Symbol {} does not exist", symbol.name));
        }

        self.symbols.insert(symbol.name.clone(), symbol.clone());
        Ok(())
    }

    /// Retrieves a symbol's configuration
    ///
    /// # Arguments
    /// * `name` - Name of the symbol to retrieve
    ///
    /// # Returns
    /// Reference to the symbol if found, None otherwise
    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Retrieves a symbol's order matcher
    ///
    /// # Arguments
    /// * `name` - Name of the symbol to retrieve matcher for
    ///
    /// # Returns
    /// Mutable reference to the matcher if found, None otherwise
    pub fn get_matcher(&mut self, name: &str) -> Option<&mut Matcher> {
        self.matchers.get_mut(name)
    }

    /// Lists all available trading symbols
    ///
    /// # Returns
    /// Vector of references to all symbol configurations
    pub fn list_symbols(&self) -> Vec<&Symbol> {
        self.symbols.values().collect()
    }

    /// Deactivates a symbol, preventing new orders but preserving existing ones
    ///
    /// # Arguments
    /// * `name` - Name of the symbol to deactivate
    ///
    /// # Returns
    /// * `Ok(())` - If symbol was deactivated successfully
    /// * `Err(String)` - If symbol does not exist
    pub fn deactivate_symbol(&mut self, name: &str) -> Result<(), String> {
        if let Some(symbol) = self.symbols.get_mut(name) {
            symbol.status = SymbolStatus::Inactive;
            Ok(())
        } else {
            Err(format!("Symbol {} does not exist", name))
        }
    }

    /// Delists a symbol, removing it from trading completely
    ///
    /// # Arguments
    /// * `name` - Name of the symbol to delist
    ///
    /// # Returns
    /// * `Ok(())` - If symbol was delisted successfully
    /// * `Err(String)` - If symbol does not exist
    pub fn delist_symbol(&mut self, name: &str) -> Result<(), String> {
        if let Some(symbol) = self.symbols.get_mut(name) {
            symbol.status = SymbolStatus::Delisted;
            self.matchers.remove(name);
            Ok(())
        } else {
            Err(format!("Symbol {} does not exist", name))
        }
    }

    /// Retrieves both a symbol's configuration and its matcher
    ///
    /// # Arguments
    /// * `name` - Name of the symbol
    ///
    /// # Returns
    /// Tuple of references to the symbol and its matcher if found, None otherwise
    pub fn get_symbol_and_matcher(&mut self, name: &str) -> Option<(&Symbol, &mut Matcher)> {
        let symbol = self.symbols.get(name)?;
        let matcher = self.matchers.get_mut(name)?;
        Some((symbol, matcher))
    }
}
