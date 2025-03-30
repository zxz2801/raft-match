use crate::server;
use pb::raft_service_server::RaftService;
use pb::{PostDataRequest, PostDataResponse, ResultCode};
use protobuf::Message;
use raft::prelude::Message as RaftMessage;

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
        request: tonic::Request<PostDataRequest>,
    ) -> Result<tonic::Response<PostDataResponse>, tonic::Status> {
        let mut response: PostDataResponse = PostDataResponse::default();
        for data in request.into_inner().data {
            match RaftMessage::parse_from_bytes(data.as_slice()) {
                Ok(message) => match server::instance().lock().await.in_mailbox.send(message) {
                    Ok(_) => {
                        response.push_ret(ResultCode::Ok);
                    }
                    Err(e) => {
                        log::warn!("raft send error: {}", e);
                        response.push_ret(ResultCode::Fail);
                        continue;
                    }
                },
                Err(e) => {
                    log::warn!("raft parse error: {}", e);
                    response.push_ret(ResultCode::Fail);
                    continue;
                }
            }
        }
        Ok(tonic::Response::new(response))
    }
}
