pub mod block;
pub mod req;
pub mod resp;
pub mod event;

use std::{
    collections::{hash_map::{self, DefaultHasher}, HashMap, HashSet},
    hash::{Hash, Hasher},
    time::{Duration, Instant}
};
use clap::{Arg, Parser};
use futures::{prelude::*, channel::oneshot};
use anyhow::{Result as AResult, bail};
use futures::{executor::block_on, FutureExt};
use libp2p::{
    bytes::{BufMut, Buf,},
    pnet::{PnetConfig, PreSharedKey},
    identify,
    identity::{self, Keypair as Keypair, PublicKey},
    core::{upgrade},
    kad::{self},
    noise, yamux, 
    futures::StreamExt,
    gossipsub::{self, Event as GSEvent, Topic as GSTopic},
    floodsub::{self, Topic as FSTopic},
    multiaddr::Protocol,
    request_response::{
        self, OutboundRequestId, ResponseChannel,
    },
    // noise::{Keypair, NoiseConfig, X25519Spec},
    tcp::{
        self, Transport, Config,
        tokio::{Tcp, TcpStream},
    },
    swarm::{SwarmEvent, Swarm}, 
};
use tokio::{
    sync::mpsc,
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    time::sleep,
};
use tracing::{Level, level_filters::LevelFilter, instrument::WithSubscriber, Subscriber};
use tracing_subscriber::{EnvFilter, fmt::SubscriberBuilder};

use self::{resp::ChainResponse, req::LocalChainReq};
pub use self::{
    block::*
};

use serde::{Serialize, Deserialize};
use once_cell::sync::Lazy as Lazy;
use libp2p::{
    PeerId,
    request_response::{ProtocolSupport},
    swarm::{NetworkBehaviour, 
        behaviour::{
            NotifyHandler,
            ConnectionClosed,
            ConnectionEstablished,
        }, 
    },
    floodsub::{Floodsub, FloodsubEvent, Topic},
    StreamProtocol, 
};

pub static KEYS: Lazy<Keypair> = Lazy::new(Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));


#[derive(NetworkBehaviour)]
pub struct Behaviour {
    // relay_client: libp2p::relay::client::Behaviour,
    pub kad: libp2p::kad::Behaviour<libp2p::kad::store::MemoryStore>,
    pub ping: libp2p::ping::Behaviour,
    pub identify: libp2p::identify::Behaviour,
    pub relay: libp2p::relay::Behaviour,
    pub rs: libp2p::rendezvous::server::Behaviour,
    pub gs: libp2p::gossipsub::Behaviour,
    pub dcutr: libp2p::dcutr::Behaviour,
    pub fs: libp2p::floodsub::Floodsub,
    pub mdns: libp2p::mdns::tokio::Behaviour,
    pub an: libp2p::autonat::Behaviour,
    pub upnp: libp2p::upnp::tokio::Behaviour,
    // #[behaviour(ignore)]
    // pub resp_sender: mpsc::UnboundedSender<ChainResponse>,
    // #[behaviour(ignore)]
    // pub init_sender: mpsc::UnboundedSender<bool>,
    // #[behaviour(ignore)]
    // pub chain: Chain,
    pub req_resp: request_response::cbor::Behaviour<LocalChainReq, ChainResponse>,
}

impl Behaviour {
    fn gs(pk: &PublicKey) -> libp2p::gossipsub::Behaviour {
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            libp2p::gossipsub::MessageId::from(s.finish().to_string())
        };
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
            .max_transmit_size(262144)
            .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
            .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
            .build()
            .map_err(|msg| std::io::Error::new(std::io::ErrorKind::Other, msg)) // Temporary hack because `build` does not return a proper `std::error::Error`
            .unwrap();
        gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(KEYS.clone()),
            gossipsub_config,
        ).unwrap()
    }

    fn kad(k: &PeerId) -> libp2p::kad::Behaviour<libp2p::kad::store::MemoryStore> {
        let mut cfg = libp2p::kad::Config::default();
        cfg.set_query_timeout(Duration::from_secs(5 * 60));
        let store = libp2p::kad::store::MemoryStore::new(k.clone(),);
        libp2p::kad::Behaviour::new(
            k.clone(),
            store,
        )
    }

    fn mdns(k: &PeerId) -> libp2p::mdns::tokio::Behaviour {
        libp2p::mdns::tokio::Behaviour::new(
            libp2p::mdns::Config::default(),
            k.clone(),
        ).unwrap()
    }

    fn ping(k: &PeerId) -> libp2p::ping::Behaviour {
        libp2p::ping::Behaviour::new(libp2p::ping::Config::new())
    }

    fn req_resp(k: &PeerId) -> request_response::cbor::Behaviour<LocalChainReq, ChainResponse> {
        request_response::cbor::Behaviour::<LocalChainReq, ChainResponse>::new(
            [
                (StreamProtocol::new("/chain/client/1"), ProtocolSupport::Outbound),
                (StreamProtocol::new("/chain/1"), ProtocolSupport::Full),
                (StreamProtocol::new("/chain/server/1"), ProtocolSupport::Inbound)
            ],
            request_response::Config::default(),
        )
    }

    fn dcutr(k: &PeerId) -> libp2p::dcutr::Behaviour {
        libp2p::dcutr::Behaviour::new(k.clone())
    }

    fn fs(k: &PeerId) -> libp2p::floodsub::Floodsub {
        libp2p::floodsub::Floodsub::new(k.clone())
    }

    fn identify(pk: &identity::PublicKey) -> libp2p::identify::Behaviour {
        libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
            "/TODO/0.0.1".to_string(),
            pk.clone(),
        ))
    }
    fn upnp(pk: &PeerId) -> libp2p::upnp::tokio::Behaviour {
        libp2p::upnp::tokio::Behaviour::default()
    }
    fn an(k: &PeerId) -> libp2p::autonat::Behaviour {
        libp2p::autonat::Behaviour::new(
            k.clone(),
            libp2p::autonat::Config::default(),
        )
    }
    fn rs(k: &PeerId) -> libp2p::rendezvous::server::Behaviour {
        libp2p::rendezvous::server::Behaviour::new(
            libp2p::rendezvous::server::Config::default(),
        )
    }
    pub fn relay(k: &PeerId) -> libp2p::relay::Behaviour {
        libp2p::relay::Behaviour::new(
            k.clone(),
            libp2p::relay::Config::default(),
        )
    }

}
impl From<Keypair> for Behaviour {
    fn from(kp:  Keypair)  -> Self {
        let pk = kp.public();
        Self {
            gs: Self::gs(&pk),
            fs: Self::fs(&pk.to_peer_id()),
            relay: Self::relay(&pk.to_peer_id()),
            kad: Self::kad(&pk.to_peer_id()),
            mdns: Self::mdns(&pk.to_peer_id()),
            dcutr: Self::dcutr(&pk.to_peer_id()),
            ping: Self::ping(&pk.to_peer_id()),
            identify: Self::identify(&pk),
            upnp: Self::upnp(&pk.to_peer_id()),
            an: Self::an(&pk.to_peer_id()),
            rs: Self::rs(&pk.to_peer_id()),
            req_resp: Self::req_resp(&pk.to_peer_id()),
        }
    }
}
impl Default for Behaviour {
    fn default() -> Self {
        Self::from(KEYS.clone())
    }
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

async fn get_psk(path: &std::path::Path) -> tokio::io::Result<Option<String>> {
    let swarm_key_file = path.join("swarm.key");
    match tokio::fs::read_to_string(swarm_key_file).await {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}