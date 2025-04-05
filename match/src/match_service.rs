//! Match service implementation
//!
//! This module implements the gRPC service for order matching operations.

use std::str::FromStr;

use pb::match_service_server::MatchService;
use pb::{
    CancelOrderRequest, CancelOrderResponse, CreateSymbolRequest, CreateSymbolResponse,
    PlaceOrderRequest, PlaceOrderResponse, QueryOrderRequest, QueryOrderResponse,
    RemoveSymbolRequest, RemoveSymbolResponse,
};
use rust_decimal::Decimal;

use crate::engine::entry::Order;
use crate::engine::entry::Symbol;
use crate::engine::matchengine::MatchCmd;
use crate::raft::proposal::Proposal;
use crate::server;

/// Protocol buffer definitions for match service
#[allow(clippy::module_inception)]
pub mod pb {
    tonic::include_proto!("r#match");
}

/// Match service implementation
#[derive(Debug, Default)]
pub struct MatchServiceSVC {}

#[tonic::async_trait]
impl MatchService for MatchServiceSVC {
    /// Queries an order's status
    ///
    /// # Arguments
    ///
    /// * `_request` - Query order request
    ///
    /// # Returns
    ///
    /// Returns the order status or an error
    async fn query_order(
        &self,
        _request: tonic::Request<QueryOrderRequest>,
    ) -> Result<tonic::Response<QueryOrderResponse>, tonic::Status> {
        todo!()
    }

    /// Places a new order
    ///
    /// This method:
    /// 1. Converts the request to a match engine order
    /// 2. Creates a match command
    /// 3. Proposes the command through Raft
    /// 4. Waits for consensus
    ///
    /// # Arguments
    ///
    /// * `request` - Place order request
    ///
    /// # Returns
    ///
    /// Returns a response indicating success or failure
    async fn place_order(
        &self,
        request: tonic::Request<PlaceOrderRequest>,
    ) -> Result<tonic::Response<PlaceOrderResponse>, tonic::Status> {
        log::info!("place order {:?}", request.get_ref());
        if let Some(order) = &request.get_ref().order {
            let order_side = match order.order_side() {
                crate::match_service::pb::OrderSide::Buy => crate::engine::entry::OrderSide::Buy,
                crate::match_service::pb::OrderSide::Sell => crate::engine::entry::OrderSide::Sell,
            };
            let order_type = match order.order_type() {
                crate::match_service::pb::OrderType::Limit => {
                    crate::engine::entry::OrderType::Limit
                }
                crate::match_service::pb::OrderType::LimitMaker => {
                    crate::engine::entry::OrderType::Limit
                }
                crate::match_service::pb::OrderType::Market => {
                    crate::engine::entry::OrderType::Market
                }
            };
            let match_order = Order::new(
                order.order_id.to_string(),
                order.symbol.clone(),
                order_type,
                order_side,
                order.price.clone(),
                order.quantity.clone(),
            );
            let cmd = MatchCmd {
                cmd: crate::engine::matchengine::MatchCmdType::PlaceOrder,
                order: Some(match_order),
                symbol: None,
            };
            let data =
                bincode::serialize(&cmd).map_err(|_| tonic::Status::internal("serialize error"))?;
            let (proposal, rx) = Proposal::normal(data.clone());
            {
                let mut server = server::instance().lock().await;
                server.add_proposal(proposal).await;
            }
            let _ = rx
                .await
                .map_err(|_| tonic::Status::internal("raft error"))?;
        };

        Ok(tonic::Response::new(PlaceOrderResponse {
            ret: 0,
            message: "ok".to_string(),
        }))
    }

    /// Cancels an existing order
    ///
    /// This method:
    /// 1. Creates a cancel order command
    /// 2. Proposes the command through Raft
    /// 3. Waits for consensus
    ///
    /// # Arguments
    ///
    /// * `request` - Cancel order request
    ///
    /// # Returns
    ///
    /// Returns a response indicating success or failure
    async fn cancel_order(
        &self,
        request: tonic::Request<CancelOrderRequest>,
    ) -> Result<tonic::Response<CancelOrderResponse>, tonic::Status> {
        log::info!("cancel order {:?}", request.get_ref());
        let order_id = request.get_ref().order_id;

        let mut match_order = Order::default();
        match_order.id = order_id.to_string();
        match_order.symbol = request.get_ref().symbol.clone();

        let cmd = MatchCmd {
            cmd: crate::engine::matchengine::MatchCmdType::CancelOrder,
            order: Some(match_order),
            symbol: None,
        };

        let data =
            bincode::serialize(&cmd).map_err(|_| tonic::Status::internal("serialize error"))?;
        let (proposal, rx) = Proposal::normal(data);
        server::instance().lock().await.add_proposal(proposal).await;
        rx.await
            .map_err(|_| tonic::Status::internal("raft error"))?;
        Ok(tonic::Response::new(CancelOrderResponse {
            ret: 0,
            message: "ok".to_string(),
        }))
    }

    /// Creates a new trading symbol
    ///
    /// This method:
    /// 1. Validates symbol parameters
    /// 2. Creates a new symbol
    /// 3. Proposes the creation through Raft
    /// 4. Waits for consensus
    ///
    /// # Arguments
    ///
    /// * `request` - Create symbol request
    ///
    /// # Returns
    ///
    /// Returns a response indicating success or failure
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
            cmd: crate::engine::matchengine::MatchCmdType::CreateSymbol,
            order: None,
            symbol: Some(match_symbol),
        };
        let data =
            bincode::serialize(&cmd).map_err(|_| tonic::Status::internal("serialize error"))?;
        let (proposal, rx) = Proposal::normal(data);
        server::instance().lock().await.add_proposal(proposal).await;
        rx.await
            .map_err(|e| tonic::Status::internal(format!("raft error: {:?}", e)))?;
        Ok(tonic::Response::new(CreateSymbolResponse {
            ret: 0,
            message: "ok".to_string(),
        }))
    }

    /// Removes a trading symbol
    ///
    /// This method:
    /// 1. Validates symbol parameters
    /// 2. Creates a remove symbol command
    /// 3. Proposes the removal through Raft
    /// 4. Waits for consensus
    ///
    /// # Arguments
    ///
    /// * `request` - Remove symbol request
    ///
    /// # Returns
    ///
    /// Returns a response indicating success or failure
    async fn remove_symbol(
        &self,
        request: tonic::Request<RemoveSymbolRequest>,
    ) -> Result<tonic::Response<RemoveSymbolResponse>, tonic::Status> {
        let match_symbol = Symbol {
            name: request.get_ref().symbol.clone(),
            ..Default::default()
        };
        let cmd = MatchCmd {
            cmd: crate::engine::matchengine::MatchCmdType::RemoveSymbol,
            order: None,
            symbol: Some(match_symbol),
        };
        let data =
            bincode::serialize(&cmd).map_err(|_| tonic::Status::internal("serialize error"))?;
        let (proposal, rx) = Proposal::normal(data);
        server::instance().lock().await.add_proposal(proposal).await;
        rx.await
            .map_err(|_| tonic::Status::internal("raft error"))?;
        Ok(tonic::Response::new(RemoveSymbolResponse {
            ret: 0,
            message: "ok".to_string(),
        }))
    }
}
