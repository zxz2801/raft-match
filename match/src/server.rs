//! Server implementation for the Raft match service
//!
//! This module implements the main server that coordinates Raft consensus,
//! gRPC services, and metrics collection.

use crate::match_service::pb::match_service_server::MatchServiceServer;
use crate::match_service::MatchServiceSVC;
use crate::metrics;
use crate::raft_service::pb::raft_service_server::RaftServiceServer;
use crate::raft_service::RaftServiceSVC;
use crate::{config, state_match};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response};
use prometheus::{Encoder, TextEncoder};
use raft::eraftpb::Message;
use std::sync::Arc;

use crate::raft::proposal::Proposal;
use crate::raft_client;
use once_cell::sync::OnceCell;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;

/// Global server instance
static INSTANCE: OnceCell<Mutex<Server>> = OnceCell::new();

/// Returns a reference to the global server instance
pub fn instance() -> &'static Mutex<Server> {
    INSTANCE.get_or_init(|| Mutex::new(Server::builder()))
}

/// Main server struct that coordinates all services
pub struct Server {
    /// Channel for receiving messages from other nodes
    pub(crate) in_mailbox: Sender<Message>,
    /// Channel for receiving proposals from clients
    pub(crate) tx_proposals: Sender<Proposal>,
}

impl Server {
    /// Creates a new server instance
    ///
    /// This method:
    /// 1. Sets up channels for message passing
    /// 2. Initializes the Raft node
    /// 3. Starts the outbound message handler
    fn builder() -> Self {
        let (tx_proposals, rx_proposals) = mpsc::channel(1000);
        let state_match = state_match::StateMatch::new();
        let id = config::instance().lock().unwrap().id;
        let start_with_leader = config::instance().lock().unwrap().start_with_leader;
        let base_path = config::instance().lock().unwrap().base_path.clone();
        let (in_mailbox, rx) = mpsc::channel(10000);
        let out_mailbox = crate::raft::node::Node::start_raft(
            start_with_leader,
            id,
            rx,
            rx_proposals,
            state_match,
            &base_path,
        );
        Self::start_run_out_message(out_mailbox);
        Server {
            in_mailbox,
            tx_proposals,
        }
    }

    /// Initializes the logger
    async fn init_logger(&mut self) {}

    /// Starts all server components
    ///
    /// This method:
    /// 1. Initializes the logger
    /// 2. Starts the gRPC server
    /// 3. Starts the metrics server
    /// 4. Initializes follower nodes
    pub async fn start(&mut self) {
        self.init_logger().await;
        self.start_grpc_server().await;
        self.start_metrics_server().await;
        self.init_followers().await;
    }

    /// Stops the server
    pub fn stop(&mut self) {
        log::info!("server stop");
    }

    /// Adds a new proposal to the server
    ///
    /// # Arguments
    ///
    /// * `proposal` - The proposal to add
    pub async fn add_proposal(&mut self, proposal: Proposal) {
        let _ = self.tx_proposals.send(proposal).await;
    }

    /// Starts the gRPC server
    ///
    /// This method:
    /// 1. Binds to the configured address
    /// 2. Registers Raft and Match services
    /// 3. Starts serving requests
    async fn start_grpc_server(&mut self) {
        let addr = config::instance()
            .lock()
            .unwrap()
            .addr
            .as_str()
            .parse()
            .unwrap();
        let mut server = tonic::transport::Server::builder();
        let raft_service = RaftServiceSVC::default();
        let match_service = MatchServiceSVC::default();
        let grpc_server = server
            .add_service(RaftServiceServer::new(raft_service))
            .add_service(MatchServiceServer::new(match_service))
            .serve(addr);
        tokio::spawn(async move {
            tokio::pin!(grpc_server);
            grpc_server.await.unwrap();
        });
        log::info!("grpc server started on {}", addr);
    }

    /// Starts the metrics server
    ///
    /// This method:
    /// 1. Binds to the configured metrics address
    /// 2. Sets up the metrics endpoint
    /// 3. Starts serving metrics requests
    async fn start_metrics_server(&mut self) {
        let addr = config::instance()
            .lock()
            .unwrap()
            .metrics_addr
            .as_str()
            .parse()
            .unwrap();
        let make_svc = make_service_fn(move |_| {
            let registry = metrics::REGISTRY_INSTANCE.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |_: Request<Body>| {
                    let registry = registry.clone();
                    async move {
                        let encoder = TextEncoder::new();
                        let metric_families = registry.gather();
                        let mut buffer = Vec::new();
                        encoder.encode(&metric_families, &mut buffer).unwrap();
                        Ok::<_, hyper::Error>(Response::new(Body::from(buffer)))
                    }
                }))
            }
        });
        metrics::init_registry();
        let server = hyper::Server::bind(&addr).serve(make_svc);
        tokio::spawn(async move {
            tokio::pin!(server);
            server.await.unwrap()
        });
        log::info!("metrics server started on {}", addr);
    }

    /// Starts the outbound message handler
    ///
    /// This method:
    /// 1. Creates a new runtime
    /// 2. Initializes the Raft client
    /// 3. Processes outbound messages
    ///
    /// # Arguments
    ///
    /// * `out_mailbox` - Channel for receiving outbound messages
    fn start_run_out_message(mut out_mailbox: Receiver<Message>) {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let client = Arc::new(Mutex::new(raft_client::RaftClient::builder()));
                while let Some(msg) = out_mailbox.recv().await {
                    let raft_client = client.lock().await;
                    raft_client.post_data(msg).await;
                }
            });
        });
    }

    /// Initializes follower nodes
    ///
    /// This method:
    /// 1. Checks if the current node is a leader
    /// 2. Gets the list of follower IDs
    /// 3. Sends add follower proposals
    async fn init_followers(&self) {
        let is_leader = config::instance().lock().unwrap().start_with_leader;
        if !is_leader {
            return;
        }

        let self_id = config::instance().lock().unwrap().id;
        let ids: Vec<u64> = config::instance()
            .lock()
            .unwrap()
            .node_list
            .iter()
            .map(|n| n.id)
            .collect();
        let ids = ids.iter().filter(|i| **i != self_id).cloned().collect();

        let proposals = self.tx_proposals.clone();
        tokio::spawn(async move {
            // wait node init
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            crate::raft::node::add_all_followers(ids, &proposals).await;
        });
    }
}
