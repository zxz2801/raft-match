use crate::config;
use pb::raft_service_client::RaftServiceClient;
use pb::{PostDataRequest, PostDataResponse};
use protobuf::Message;
use raft::prelude::Message as RaftMessage;
use std::sync::Arc;
use tokio::sync::Mutex;

#[allow(clippy::module_inception)]
pub mod pb {
    tonic::include_proto!("raft");
}

pub struct RaftClient {
    pub channels:
        Arc<Mutex<std::collections::HashMap<u64, RaftServiceClient<tonic::transport::Channel>>>>,
}

impl RaftClient {
    pub fn builder() -> RaftClient {
        RaftClient {
            channels: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn post_data(
        &self,
        data: RaftMessage,
    ) -> impl std::future::Future<Output = PostDataResponse> + Send {
        let channels = self.channels.clone();
        async move {
            let mut request: PostDataRequest = PostDataRequest::default();
            request.data.push(data.write_to_bytes().unwrap());
            log::info!("raft client post data: {}", request.data.len());

            // Try to get existing channel or create new one
            let mut client = {
                let mut channels = channels.lock().await;
                if let Some(client) = channels.get_mut(&data.to) {
                    client.clone()
                } else {
                    let addr = config::instance().lock().unwrap().node_list[data.to as usize - 1]
                        .addr
                        .clone();
                    match RaftServiceClient::connect(addr.clone()).await {
                        Ok(channel) => {
                            channels.insert(data.to, channel.clone());
                            channel
                        }
                        Err(e) => {
                            log::error!("raft client connect error: {}", e);
                            return PostDataResponse::default();
                        }
                    }
                }
            };

            // Try to send request without holding the channels lock
            match client.post_data(request).await {
                Ok(response) => response.into_inner(),
                Err(e) => {
                    log::error!("raft client post data error: {}", e);
                    PostDataResponse::default()
                }
            }
        }
    }
}
