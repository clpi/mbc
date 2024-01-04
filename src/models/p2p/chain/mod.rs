pub mod block;

pub use self::{
    block::*
};

use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;
use once_cell::sync::Lazy as Lazy;
use std::collections::HashSet;
use libp2p::{
    PeerId,
    swarm::{NetworkBehaviour, 
        behaviour::{
            NotifyHandler,
            ConnectionClosed,
            ConnectionEstablished,
        }, 
        Swarm},
    mdns::{Behaviour},
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity::Keypair, 
};

use super::{Block, block::Chain};


pub static KEYS: Lazy<Keypair> = Lazy::new(Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));

#[derive(Serialize, Debug, Deserialize)]
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub recv: String,
}


#[derive(Serialize, Debug, Deserialize)]
pub struct LocalChainReq {
    pub from_peer_id: String,
}

pub enum EventType {
    LocalChainResponse(ChainResponse),
    Input(String),
    Init,
}

// #[derive(NetworkBehaviour)]
pub struct AppBehavior {
    pub floodsub: Floodsub,
    // #[behaviour(ignore)]
    pub resp_sender: mpsc::UnboundedSender<ChainResponse>,
    // #[behaviour(ignore)]
    pub init_sender: mpsc::UnboundedSender<bool>,
    // #[behaviour(ignore)]
    pub chain: Chain,
}

impl AppBehavior {
    pub async fn new(
        chain: Chain,
        resp_sender: mpsc::UnboundedSender<ChainResponse>,
        init_sender: mpsc::UnboundedSender<bool>,
    ) -> Self {
        let mut beh = Self {
            floodsub: Floodsub::new(*PEER_ID),
            resp_sender,
            init_sender,
            chain,
        };
        beh.floodsub.subscribe(CHAIN_TOPIC.clone());
        beh.floodsub.subscribe(BLOCK_TOPIC.clone());
        beh
    }
}
