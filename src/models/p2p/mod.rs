pub mod client;
pub mod data;
pub mod store;
pub mod server;
pub mod chain;


use std::{
    collections::{hash_map::{self, DefaultHasher}, HashMap, HashSet},
    error::Error as SError,
    path::PathBuf,
    io::{Write},
    hash::{Hash, Hasher},
    time::{Duration, Instant}
};
use clap::{Arg, Parser};
use futures::{prelude::*, channel::oneshot};
use anyhow::{Result as AResult, bail};
use futures::{executor::block_on, FutureExt};
use libp2p::{
    StreamProtocol,
    bytes::{BufMut, Buf,},
    identify,
    identity::{self, Keypair as KeyP, ed25519::Keypair},
    core::{upgrade},
    kad::{self},
    noise, yamux, 
    futures::StreamExt,
    gossipsub::{self, Event as GSEvent, Topic as GSTopic},
    floodsub::{self, Topic as FSTopic},
    multiaddr::Protocol,
    request_response::{
        self, OutboundRequestId, ProtocolSupport, ResponseChannel,
    },
    // noise::{Keypair, NoiseConfig, X25519Spec},
    tcp::{
        self, Transport, Config,
        tokio::{Tcp, TcpStream},
    },
    swarm::{SwarmEvent, NetworkBehaviour, Swarm}, floodsub::Floodsub, PeerId, Multiaddr,
};
use tokio::{
    sync::mpsc,
};

pub type Sender<T> = oneshot::Sender<T>;
pub type SendResult<T> = Sender<Result<T, Box<dyn SError>>>;
 use self::{resp::ChainResponse, req::{LocalChainReq, Command}};
pub use self::{
    chain::*,
    // client::Client,
    data::*,
    store::Store,
    server::Server,
 };


#[derive(Clone)]
pub struct Client {
    _send: mpsc::Sender<Command>,
    _addr: libp2p::Multiaddr,
}

pub fn get_list_peers() -> Vec<String> {
    log::info!("Getting list of peers");
    let peers = Vec::<String>::new();
    let mut unique = HashSet::new();
    for peer in peers {
        unique.insert(peer);
    }
    unique.iter().map(|p| p.to_string()).collect()
}

pub fn get_chain() ->  String {
    log::info!("Getting chain");
    let chain = serde_json::to_string_pretty(
        ""
    ).unwrap();
    log::info!("Chain: {}", chain);
    chain
}

pub fn hadle_create_block(cmd: &str) {
    if let Some(d) = cmd.strip_prefix("create b") {
        // let latest_b = 
    }
}

pub fn init_tcp_conf() -> libp2p::tcp::Config {
    libp2p::tcp::Config::default()
        .port_reuse(true)
        .nodelay(true)
}

pub async fn init_swarm() -> Result<Swarm<chain::Behaviour>, Box<dyn SError>> {
    let auth_keys = KeyP::generate_ed25519();
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(auth_keys.clone())
        .with_tokio()
        .with_tcp(
            init_tcp_conf(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_dns()?
        .with_behaviour(|k|
            chain::Behaviour::default()
        )?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(5)))
        .build();
    swarm.behaviour_mut()
        .kad
        .set_mode(Some(libp2p::kad::Mode::Server));
    let gst = gossipsub::IdentTopic::new("test-net");
    let fst = floodsub::Topic::new("chain");
    swarm.behaviour_mut().fs.subscribe(fst);
    swarm.behaviour_mut().gs.subscribe(&gst)?;
    // let mut stdin = std::io::BufReader::new(std::io::stdin()).lines();


    Ok(swarm)
}

pub fn gen_ed25519(seed: u8) -> libp2p::identity::Keypair {
    let mut byt = [0u8; 32];
    byt[0] = seed;
    libp2p::identity::Keypair::ed25519_from_bytes(byt)
        .expect("Failed to generate keypair wrong leng")
}