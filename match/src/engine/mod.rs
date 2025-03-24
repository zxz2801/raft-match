use bincode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MatchEngine {
    order_book: std::collections::BTreeMap<String, String>,
}

impl MatchEngine {
    pub fn new() -> MatchEngine {
        MatchEngine {
            order_book: std::collections::BTreeMap::new(),
        }
    }

    pub fn on_message(&mut self, data: &[u8]) {
        // 转换成具体消息 再处理
        self.order_book
            .insert("test".to_string(), "value".to_string());
    }

    pub fn on_snapshot(&mut self, data: &[u8]) {
        // 从快照恢复数据
        *self = bincode::deserialize(data).unwrap();
    }

    pub fn snapshot(&self) -> Vec<u8> {
        // 返回快照数据
        bincode::serialize(self).unwrap()
    }
}
