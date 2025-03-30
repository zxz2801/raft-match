use std::str::FromStr;

use pb::match_service_server::MatchService;
use pb::{
    CancelOrderRequest, CancelOrderResponse, CreateSymbolRequest, CreateSymbolResponse,
    PlaceOrderRequest, PlaceOrderResponse, QueryOrderRequest, QueryOrderResponse,
    RemoveSymbolRequest, RemoveSymbolResponse
};
use rust_decimal::Decimal;

use crate::engine::engine::MatchCmd;
use crate::engine::entry::Order;
use crate::engine::entry::Symbol;
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
        _request: tonic::Request<QueryOrderRequest>,
    ) -> Result<tonic::Response<QueryOrderResponse>, tonic::Status> {
        todo!()
    }

    async fn place_order(
        &self,
        request: tonic::Request<PlaceOrderRequest>,
    ) -> Result<tonic::Response<PlaceOrderResponse>, tonic::Status> {
        log::info!("place order {:?}", request.get_ref());
        match &request.get_ref().order {
            Some(order) => {
                let price = Decimal::from_str(&order.price)
                    .map_err(|_| tonic::Status::invalid_argument("invalid price"))?;
                let quantity = Decimal::from_str(&order.quantity)
                    .map_err(|_| tonic::Status::invalid_argument("invalid quantity"))?;
                let order_side = match order.order_side() {
                    crate::match_service::pb::OrderSide::Buy => crate::engine::entry::OrderSide::Buy,
                    crate::match_service::pb::OrderSide::Sell => crate::engine::entry::OrderSide::Sell,
                };
                let order_type = match order.order_type() {
                    crate::match_service::pb::OrderType::Limit => crate::engine::entry::OrderType::Limit,
                    crate::match_service::pb::OrderType::LimitMaker => {
                        crate::engine::entry::OrderType::Limit
                    }
                    crate::match_service::pb::OrderType::Market => crate::engine::entry::OrderType::Market,
                };
                let match_order = Order::new(
                    order.order_id.to_string(),
                    order.symbol.clone(),
                    order_type,
                    order_side,
                    price,
                    quantity,
                );
                let cmd = MatchCmd {
                    cmd: crate::engine::engine::MatchCmdType::PlaceOrder,
                    order: Some(match_order),
                    symbol: None,
                };
                let data = bincode::serialize(&cmd)
                    .map_err(|_| tonic::Status::internal("serialize error"))?;
                let (proposal, rx) = Proposal::normal(data);
                server::instance().lock().await.add_proposal(proposal).await;
                rx.recv()
                    .map_err(|_| tonic::Status::internal("raft error"))?;
            }
            None => {}
        };

        Ok(tonic::Response::new(PlaceOrderResponse {
            ret: 0,
            message: "ok".to_string(),
        }))
    }

    async fn cancel_order(
        &self,
        request: tonic::Request<CancelOrderRequest>,
    ) -> Result<tonic::Response<CancelOrderResponse>, tonic::Status> {
        log::info!("cancel order {:?}", request.get_ref());
        let order_id = request.get_ref().order_id.clone();

        let mut match_order = Order::default();
        match_order.id = order_id.to_string();
        match_order.symbol = request.get_ref().symbol.clone();

        let cmd = MatchCmd {
            cmd: crate::engine::engine::MatchCmdType::CancelOrder,
            order: Some(match_order),
            symbol: None,
        };

        let data = bincode::serialize(&cmd)
            .map_err(|_| tonic::Status::internal("serialize error"))?;
        let (proposal, rx) = Proposal::normal(data);
        server::instance().lock().await.add_proposal(proposal).await;
        rx.recv()
            .map_err(|_| tonic::Status::internal("raft error"))?;
        Ok(tonic::Response::new(CancelOrderResponse {
            ret: 0,
            message: "ok".to_string(),
        }))
    }

    async fn create_symbol(
        &self,
        request: tonic::Request<CreateSymbolRequest>,
    ) -> Result<tonic::Response<CreateSymbolResponse>, tonic::Status> {
        let symbol = request.get_ref().symbol.as_ref().unwrap();
        let min_quantity = Decimal::from_str(&symbol.min_quantity)
            .map_err(|_| tonic::Status::invalid_argument("invalid min quantity"))?;
        let max_quantity = Decimal::from_str(&symbol.max_quantity)
            .map_err(|_| tonic::Status::invalid_argument("invalid max quantity"))?;
        let min_amount = Decimal::from_str(&symbol.min_amount)
            .map_err(|_| tonic::Status::invalid_argument("invalid min amount"))?;
        let max_amount = Decimal::from_str(&symbol.max_amount)
            .map_err(|_| tonic::Status::invalid_argument("invalid max amount"))?;
        let match_symbol = Symbol::new(
            symbol.symbol.clone(),
            symbol.base.clone(),
            symbol.quote.clone(),
            symbol.price_precision,
            symbol.quantity_precision,
            min_quantity,
            max_quantity,
            min_amount,
            max_amount,
        );
        let cmd = MatchCmd {
            cmd: crate::engine::engine::MatchCmdType::CreateSymbol,
            order: None,
            symbol: Some(match_symbol),
        };
        let data = bincode::serialize(&cmd)
            .map_err(|_| tonic::Status::internal("serialize error"))?;
        let (proposal, rx) = Proposal::normal(data);
        server::instance().lock().await.add_proposal(proposal).await;
        rx.recv()
            .map_err(|_| tonic::Status::internal("raft error"))?;
        Ok(tonic::Response::new(CreateSymbolResponse {
            ret: 0,
            message: "ok".to_string(),
        }))
    }

    async fn remove_symbol(
        &self,
        request: tonic::Request<RemoveSymbolRequest>,
    ) -> Result<tonic::Response<RemoveSymbolResponse>, tonic::Status> {
        let symbol = request.get_ref().symbol.as_ref().unwrap();
        let min_quantity = Decimal::from_str(&symbol.min_quantity)
            .map_err(|_| tonic::Status::invalid_argument("invalid min quantity"))?;
        let max_quantity = Decimal::from_str(&symbol.max_quantity)
            .map_err(|_| tonic::Status::invalid_argument("invalid max quantity"))?;
        let min_amount = Decimal::from_str(&symbol.min_amount)
            .map_err(|_| tonic::Status::invalid_argument("invalid min amount"))?;
        let max_amount = Decimal::from_str(&symbol.max_amount)
            .map_err(|_| tonic::Status::invalid_argument("invalid max amount"))?;
        let match_symbol = Symbol::new(
            symbol.symbol.clone(),
            symbol.base.clone(),
            symbol.quote.clone(),
            symbol.price_precision,
            symbol.quantity_precision,
            min_quantity,
            max_quantity,
            min_amount,
            max_amount,
        );
        let cmd = MatchCmd {
            cmd: crate::engine::engine::MatchCmdType::RemoveSymbol,
            order: None,
            symbol: Some(match_symbol),
        };
        let data = bincode::serialize(&cmd)
            .map_err(|_| tonic::Status::internal("serialize error"))?;
        let (proposal, rx) = Proposal::normal(data);
        server::instance().lock().await.add_proposal(proposal).await;
        rx.recv()
            .map_err(|_| tonic::Status::internal("raft error"))?;
        Ok(tonic::Response::new(RemoveSymbolResponse {
            ret: 0,
            message: "ok".to_string(),
        }))
    }
}
