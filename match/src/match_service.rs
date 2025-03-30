use pb::match_service_server::MatchService;
use pb::{
    CancelOrderRequest, CancelOrderResponse, CreateSymbolRequest, CreateSymbolResponse,
    PlaceOrderRequest, PlaceOrderResponse, QueryOrderRequest, QueryOrderResponse,
    RemoveSymbolRequest, RemoveSymbolResponse, UpdateSymbolRequest, UpdateSymbolResponse,
};

use crate::raft::proposal::Proposal;
use crate::server;

#[allow(clippy::module_inception)]
pub mod pb {
    tonic::include_proto!("r#match");
}

#[derive(Debug, Default)]
pub struct MatchServiceSVC {}

#[tonic::async_trait]
impl MatchService for MatchServiceSVC {
    async fn query_order(
        &self,
        request: tonic::Request<QueryOrderRequest>,
    ) -> Result<tonic::Response<QueryOrderResponse>, tonic::Status> {
        todo!()
    }

    async fn place_order(
        &self,
        request: tonic::Request<PlaceOrderRequest>,
    ) -> Result<tonic::Response<PlaceOrderResponse>, tonic::Status> {
        log::info!("place order {:?}", request.get_ref());
        let (proposal, rx) = Proposal::normal("1".to_string().into_bytes());
        server::instance().lock().await.add_proposal(proposal).await;
        rx.recv()
            .map_err(|_| tonic::Status::internal("raft error"))?;
        Ok(tonic::Response::new(PlaceOrderResponse {
            ret: 0,
            message: "ok".to_string(),
        }))
    }

    async fn cancel_order(
        &self,
        request: tonic::Request<CancelOrderRequest>,
    ) -> Result<tonic::Response<CancelOrderResponse>, tonic::Status> {
        todo!()
    }

    async fn create_symbol(
        &self,
        request: tonic::Request<CreateSymbolRequest>,
    ) -> Result<tonic::Response<CreateSymbolResponse>, tonic::Status> {
        todo!()
    }

    async fn update_symbol(
        &self,
        request: tonic::Request<UpdateSymbolRequest>,
    ) -> Result<tonic::Response<UpdateSymbolResponse>, tonic::Status> {
        todo!()
    }

    async fn remove_symbol(
        &self,
        request: tonic::Request<RemoveSymbolRequest>,
    ) -> Result<tonic::Response<RemoveSymbolResponse>, tonic::Status> {
        todo!()
    }
}
