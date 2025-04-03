use crate::server;
use pb::raft_service_server::RaftService;
use pb::{PostDataRequest, PostDataResponse};
use protobuf::Message;
use raft::prelude::Message as RaftMessage;
use tonic::Streaming;
#[allow(clippy::module_inception)]
pub mod pb {
    tonic::include_proto!("raft");
}

#[derive(Debug, Default)]
pub struct RaftServiceSVC {}

#[tonic::async_trait]
impl RaftService for RaftServiceSVC {
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
