//! Raft service implementation
//!
//! This module implements the gRPC service for Raft communication between nodes.

use crate::server;
use pb::raft_service_server::RaftService;
use pb::{PostDataRequest, PostDataResponse};
use protobuf::Message;
use raft::prelude::Message as RaftMessage;
use tonic::Streaming;

/// Protocol buffer definitions for Raft service
#[allow(clippy::module_inception)]
pub mod pb {
    tonic::include_proto!("raft");
}

/// Raft service implementation
#[derive(Debug, Default)]
pub struct RaftServiceSVC {}

#[tonic::async_trait]
impl RaftService for RaftServiceSVC {
    /// Handles incoming Raft messages from other nodes
    ///
    /// This method:
    /// 1. Receives a stream of Raft messages
    /// 2. Parses each message
    /// 3. Forwards valid messages to the server's inbox
    /// 4. Logs and skips invalid messages
    ///
    /// # Arguments
    ///
    /// * `request` - Streaming request containing Raft messages
    ///
    /// # Returns
    ///
    /// Returns a response indicating success or failure
    async fn post_data(
        &self,
        request: tonic::Request<Streaming<PostDataRequest>>,
    ) -> Result<tonic::Response<PostDataResponse>, tonic::Status> {
        let mut stream = request.into_inner();
        while let Some(req) = stream.message().await? {
            match RaftMessage::parse_from_bytes(req.data.as_slice()) {
                Ok(message) => match server::instance()
                    .lock()
                    .await
                    .in_mailbox
                    .send(message)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        log::warn!("raft send error: {}", e);
                        continue;
                    }
                },
                Err(e) => {
                    log::warn!("raft parse error: {}", e);
                    continue;
                }
            }
        }
        Ok(tonic::Response::new(PostDataResponse::default()))
    }
}
