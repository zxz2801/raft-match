use crate::config;
use pb::raft_service_client::RaftServiceClient;
use pb::{PostDataRequest, PostDataResponse, ResultCode};
use protobuf::Message;
use raft::prelude::Message as RaftMessage;
use std::sync::Arc;

#[allow(clippy::module_inception)]
pub mod pb {
    tonic::include_proto!("raft");
}

pub struct RaftClient {
    pub channels: std::collections::HashMap<u64, RaftServiceClient<tonic::transport::Channel>>,
}

impl RaftClient {
    pub fn builder() -> RaftClient {
        RaftClient {
            channels: std::collections::HashMap::new(),
        }
    }

    pub async fn initialize(&mut self) {
        let config_lock = config::instance().lock().unwrap();
        for node in config_lock.node_list.iter() {
            self.channels.insert(
                node.id,
                RaftServiceClient::connect(node.addr.clone()).await.unwrap(),
            );
        }
    }

    pub async fn post_data(&mut self, data: RaftMessage) -> PostDataResponse {
        let mut request: PostDataRequest = PostDataRequest::default();
        request.data.push(data.write_to_bytes().unwrap());

        match self.channels.get_mut(&data.to) {
            Some(client) => {
                let response = client.post_data(request).await.unwrap();
                response.into_inner()
            }
            None => PostDataResponse::default(),
        }
    }
}
