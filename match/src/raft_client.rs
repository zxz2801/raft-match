//! Raft client implementation
//!
//! This module provides functionality for sending Raft messages to other nodes
//! in the cluster.

use crate::config;
use pb::raft_service_client::RaftServiceClient;
use pb::PostDataRequest;
use protobuf::Message;
use raft::prelude::Message as RaftMessage;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

/// Protocol buffer definitions for Raft service
#[allow(clippy::module_inception)]
pub mod pb {
    tonic::include_proto!("raft");
}

/// Client for a single peer node
struct PeerClient {
    /// Channel sender for sending messages to the peer
    sender: Sender<PostDataRequest>,
    /// Flag indicating if the client is invalid/needs reconnection
    invalid: Arc<AtomicBool>,
}

impl PeerClient {
    /// Creates a new peer client
    ///
    /// This method:
    /// 1. Establishes a connection to the peer
    /// 2. Creates a channel for message passing
    /// 3. Spawns a background task for streaming messages
    ///
    /// # Arguments
    ///
    /// * `addr` - Address of the peer node
    ///
    /// # Returns
    ///
    /// Returns a new PeerClient instance or an error if connection fails
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

    /// Streams messages to the peer node
    ///
    /// # Arguments
    ///
    /// * `client` - Raft service client
    /// * `receiver` - Channel receiver for incoming messages
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if successful, or an error if streaming fails
    async fn stream_messages(
        client: &mut RaftServiceClient<tonic::transport::Channel>,
        receiver: Receiver<PostDataRequest>,
    ) -> Result<(), tonic::Status> {
        let stream = tokio_stream::wrappers::ReceiverStream::new(receiver);
        let _ = client.post_data(stream).await?;
        Ok(())
    }
}

/// Client for managing connections to all peer nodes
pub struct RaftClient {
    /// Map of peer IDs to their respective clients
    peers: Arc<Mutex<std::collections::HashMap<u64, PeerClient>>>,
}

impl RaftClient {
    /// Creates a new RaftClient instance
    pub fn builder() -> RaftClient {
        RaftClient {
            peers: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Sends a Raft message to a peer node
    ///
    /// This method:
    /// 1. Gets or creates a client for the target peer
    /// 2. Checks if the client is valid
    /// 3. Sends the message through the client's channel
    ///
    /// # Arguments
    ///
    /// * `data` - The Raft message to send
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
