use crate::config;
use pb::raft_service_client::RaftServiceClient;
use pb::PostDataRequest;
use protobuf::Message;
use raft::prelude::Message as RaftMessage;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

#[allow(clippy::module_inception)]
pub mod pb {
    tonic::include_proto!("raft");
}

struct PeerClient {
    sender: Sender<PostDataRequest>,
    invalid: Arc<AtomicBool>,
}

impl PeerClient {
    async fn new(addr: String) -> Result<Self, tonic::transport::Error> {
        let client = RaftServiceClient::connect(addr).await?;
        let (sender, receiver) = mpsc::channel(1000);

        // Start background streaming task
        let mut client_clone = client.clone();
        let invalid = Arc::new(AtomicBool::new(false));
        let invalid_clone = invalid.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::stream_messages(&mut client_clone, receiver).await {
                log::error!("Streaming messages failed: {}", e);
                invalid_clone.store(true, Ordering::SeqCst);
            }
        });

        Ok(Self { sender, invalid })
    }

    async fn stream_messages(
        client: &mut RaftServiceClient<tonic::transport::Channel>,
        receiver: Receiver<PostDataRequest>,
    ) -> Result<(), tonic::Status> {
        let stream = tokio_stream::wrappers::ReceiverStream::new(receiver);
        let _ = client.post_data(stream).await?;
        Ok(())
    }
}

pub struct RaftClient {
    peers: Arc<Mutex<std::collections::HashMap<u64, PeerClient>>>,
}

impl RaftClient {
    pub fn builder() -> RaftClient {
        RaftClient {
            peers: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn post_data(&self, data: RaftMessage) {
        let peers = self.peers.clone();
        let mut peers = peers.lock().await;

        // Get or create peer client
        let peer_client = if let Some(client) = peers.get_mut(&data.to) {
            client
        } else {
            let addr = config::instance().lock().unwrap().node_list[data.to as usize - 1]
                .addr
                .clone();
            match PeerClient::new(addr).await {
                Ok(client) => {
                    peers.insert(data.to, client);
                    peers.get_mut(&data.to).unwrap()
                }
                Err(e) => {
                    log::error!("Failed to create peer client: {}", e);
                    return;
                }
            }
        };

        if peer_client.invalid.load(Ordering::SeqCst) {
            peers.remove(&data.to);
            return;
        }

        // Send message through channel
        let request = PostDataRequest {
            data: data.write_to_bytes().unwrap(),
        };
        if let Err(_e) = peer_client.sender.try_send(request) {
            // log::error!("Failed to send message to peer: {}", e);
        }
    }
}
