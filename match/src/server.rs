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
use std::collections::VecDeque;
use std::sync::Arc;

use crate::raft::proposal::Proposal;
use crate::raft_client;
use once_cell::sync::OnceCell;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;

static INSTANCE: OnceCell<Mutex<Server>> = OnceCell::new();
pub fn instance() -> &'static Mutex<Server> {
    INSTANCE.get_or_init(|| Mutex::new(Server::builder()))
}

pub struct Server {
    pub(crate) in_mailbox: Sender<Message>, // <- other node
    pub(crate) proposals: Arc<std::sync::Mutex<VecDeque<Proposal>>>, // proposals
}

impl Server {
    fn builder() -> Self {
        let proposals: Arc<std::sync::Mutex<VecDeque<Proposal>>> =
            Arc::new(std::sync::Mutex::new(VecDeque::new()));
        let state_match = state_match::StateMatch::new();
        let id = config::instance().lock().unwrap().id;
        let start_with_leader = config::instance().lock().unwrap().start_with_leader;
        let base_path = config::instance().lock().unwrap().base_path.clone();
        let (in_mailbox, rx) = mpsc::channel();
        let out_mailbox = crate::raft::node::Node::start_raft(
            start_with_leader,
            id,
            rx,
            proposals.clone(),
            state_match,
            &base_path,
        );
        Self::start_run_out_message(out_mailbox);
        Server {
            in_mailbox,
            proposals,
        }
    }

    async fn init_logger(&mut self) {}

    pub async fn start(&mut self) {
        self.init_logger().await;
        self.start_grpc_server().await;
        self.start_metrics_server().await;
        self.test_data().await;
    }

    pub fn stop(&mut self) {
        log::info!("server stop");
    }

    pub async fn add_proposal(&mut self, proposal: Proposal) {
        self.proposals.lock().unwrap().push_back(proposal);
    }
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

    fn start_run_out_message(out_mailbox: Receiver<Message>) {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let client = Arc::new(Mutex::new(raft_client::RaftClient::builder()));
                client.lock().await.initialize().await;
                while let Ok(msg) = out_mailbox.recv() {
                    let item = client.clone();
                    tokio::spawn(async move {
                        let raft_client = item.lock().await;
                        let post_data = raft_client.post_data(msg);
                        drop(raft_client); // Release the lock immediately
                        post_data.await;
                    });
                }
            });
        });
    }

    async fn test_data(&self) {
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

        let proposals = self.proposals.clone();
        tokio::spawn(async move {
            crate::raft::node::add_all_followers(ids, &proposals);
            let mut counter = 0;
            loop {
                let (proposal, _rx) = Proposal::normal(counter.to_string().into_bytes());
                proposals.lock().unwrap().push_back(proposal);
                log::info!("Added test proposal: {}", counter);
                counter += 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    }
}
