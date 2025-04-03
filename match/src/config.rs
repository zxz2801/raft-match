//! Configuration module for the Raft match service
//!
//! This module handles runtime configuration including node settings, addresses, and paths.

use log::warn;
use once_cell::sync::OnceCell;
use serde_derive::Deserialize;
use std::sync::Mutex;

/// Global configuration instance
static INSTANCE: OnceCell<Mutex<RuntimeConfig>> = OnceCell::new();

/// Returns a reference to the global configuration instance
pub fn instance() -> &'static Mutex<RuntimeConfig> {
    INSTANCE.get_or_init(|| Mutex::new(RuntimeConfig::new()))
}

/// Configuration for a single node in the Raft cluster
#[derive(Debug, Deserialize, Clone)]
pub struct NodeConfig {
    /// Unique identifier for the node
    pub id: u64,
    /// Network address of the node
    pub addr: String,
}

/// Runtime configuration for the Raft match service
#[derive(Debug, Deserialize, Clone)]
pub struct RuntimeConfig {
    /// Current node's ID
    pub id: u64,
    /// Whether to start the node as a leader
    pub start_with_leader: bool,
    /// Network address for Raft communication
    pub addr: String,
    /// Network address for metrics collection
    pub metrics_addr: String,
    /// Base path for data storage
    pub base_path: String,
    /// List of all nodes in the Raft cluster
    pub node_list: Vec<NodeConfig>,
}

impl RuntimeConfig {
    /// Creates a new RuntimeConfig with default values
    pub fn new() -> Self {
        RuntimeConfig {
            id: 1,
            start_with_leader: false,
            addr: "0.0.0.0:4000".to_string(),
            metrics_addr: "0.0.0.0:4010".to_string(),
            node_list: Vec::new(),
            base_path: "./data".to_string(),
        }
    }

    /// Loads configuration from a TOML file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML configuration file
    ///
    /// # Returns
    ///
    /// Returns Some(RuntimeConfig) if successful, or None if there was an error
    pub fn from_toml(path: &str) -> Option<Self> {
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    "Something went wrong reading the runtime config file, {:?}",
                    e
                );
                return Some(RuntimeConfig::new());
            }
        };
        let config: RuntimeConfig = match toml::from_str(&contents) {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    "Something went wrong reading the runtime config file, {:?}",
                    e
                );
                return Some(RuntimeConfig::new());
            }
        };
        instance().lock().unwrap().clone_from(&config);
        Some(config)
    }
}
