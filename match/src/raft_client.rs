use crate::config;
use pb::raft_service_client::RaftServiceClient;
use pb::{PostDataRequest, PostDataResponse};
use protobuf::Message;
use raft::prelude::Message as RaftMessage;
use tonic::transport::channel;

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
            let channel = RaftServiceClient::connect(node.addr.clone()).await;
            match channel {
                Ok(channel) => {
                    self.channels.insert(node.id, channel);
                }
                Err(e) => {
                    log::error!("raft client connect error: {}", e);       
                }
            };
        }
    }

    pub async fn post_data(&mut self, data: RaftMessage) -> PostDataResponse {
        let mut request: PostDataRequest = PostDataRequest::default();
        request.data.push(data.write_to_bytes().unwrap());
        log::info!("raft client post data: {}", request.data.len());
        match self.channels.get_mut(&data.to) {
            Some(client) => {
                let response = client.post_data(request).await;
                match response {
                    Ok(response) => {
                        response.into_inner()
                    }
                    Err(e) => {
                        log::error!("raft client connect error: {}", e);
                        PostDataResponse::default()
                    }
                }
            }
            None => {
                let addr = config::instance().lock().unwrap().node_list[data.to as usize-1].addr.clone();
                let channel = RaftServiceClient::connect(addr.clone()).await;
                match channel {
                    Ok(mut channel) => {
                        let response = channel.post_data(request).await;
                        match response {
                            Ok(response) => {
                                response.into_inner()
                            }
                            Err(e) => {
                                log::error!("raft client connect error: {}", e);
                                PostDataResponse::default()
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("raft client connect error: {}", e);
                        PostDataResponse::default()
                    }
                }
            }
        }
    }
}
