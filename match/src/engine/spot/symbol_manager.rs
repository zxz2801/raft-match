use crate::engine::entry::{Symbol, SymbolStatus};
use crate::engine::matchlogic::Matcher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SymbolManager {
    symbols: HashMap<String, Symbol>,
    matchers: HashMap<String, Matcher>,
}

#[allow(unused)]
impl SymbolManager {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            matchers: HashMap::new(),
        }
    }

    pub fn add_symbol(&mut self, symbol: Symbol) -> Result<(), String> {
        if self.symbols.contains_key(&symbol.name) {
            return Err(format!("Symbol {} already exists", symbol.name));
        }

        self.symbols.insert(symbol.name.clone(), symbol.clone());
        self.matchers
            .insert(symbol.name.clone(), Matcher::new(symbol.name.clone()));
        Ok(())
    }

    pub fn update_symbol(&mut self, symbol: Symbol) -> Result<(), String> {
        if !self.symbols.contains_key(&symbol.name) {
            return Err(format!("Symbol {} does not exist", symbol.name));
        }

        self.symbols.insert(symbol.name.clone(), symbol.clone());
        Ok(())
    }

    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    pub fn get_matcher(&mut self, name: &str) -> Option<&mut Matcher> {
        self.matchers.get_mut(name)
    }

    pub fn list_symbols(&self) -> Vec<&Symbol> {
        self.symbols.values().collect()
    }

    pub fn deactivate_symbol(&mut self, name: &str) -> Result<(), String> {
        if let Some(symbol) = self.symbols.get_mut(name) {
            symbol.status = SymbolStatus::Inactive;
            Ok(())
        } else {
            Err(format!("Symbol {} does not exist", name))
        }
    }

    pub fn delist_symbol(&mut self, name: &str) -> Result<(), String> {
        if let Some(symbol) = self.symbols.get_mut(name) {
            symbol.status = SymbolStatus::Delisted;
            self.matchers.remove(name);
            Ok(())
        } else {
            Err(format!("Symbol {} does not exist", name))
        }
    }

    pub fn get_symbol_and_matcher(&mut self, name: &str) -> Option<(&Symbol, &mut Matcher)> {
        let symbol = self.symbols.get(name)?;
        let matcher = self.matchers.get_mut(name)?;
        Some((symbol, matcher))
    }
}
