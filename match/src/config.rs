use log::{error, warn};
use once_cell::sync::OnceCell;
use serde_derive::Deserialize;
use std::sync::Mutex;

static INSTANCE: OnceCell<Mutex<RuntimeConfig>> = OnceCell::new();

pub fn instance() -> &'static Mutex<RuntimeConfig> {
    INSTANCE.get_or_init(|| Mutex::new(RuntimeConfig::new()))
}

#[derive(Debug, Deserialize, Clone)]
pub struct NodeConfig {
    pub id: u64,
    pub addr: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RuntimeConfig {
    pub id: u64,
    pub start_with_leader: bool,
    pub addr: String,
    pub metrics_addr: String,
    pub node_list: Vec<NodeConfig>,
}

impl RuntimeConfig {
    pub fn new() -> Self {
        RuntimeConfig {
            id: 1,
            start_with_leader: false,
            addr: "0.0.0.0:4000".to_string(),
            metrics_addr: "0.0.0.0:4010".to_string(),
            node_list: Vec::new(),
        }
    }

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
        instance().lock().unwrap().addr.clone_from(&config.addr);
        instance()
            .lock()
            .unwrap()
            .metrics_addr
            .clone_from(&config.metrics_addr);
        Some(config)
    }
}
