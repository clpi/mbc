pub mod client;
pub mod data;
pub mod store;
pub mod server;
pub mod chain;


use std::{
    collections::{hash_map::{self, DefaultHasher}, HashMap, HashSet},
    error::Error as SError,
    time::{Duration, Instant}
};
use futures::{prelude::*, channel::oneshot};
use libp2p::{
    gossipsub, floodsub,
    core::{upgrade::{self, Version}},
    tcp, swarm::Swarm, noise, yamux,
    Transport,
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

pub async fn init_swarm() -> Result<Swarm<chain::Behaviour>, Box<dyn SError>> {
    let auth_keys = gen_ed25519(0);
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(auth_keys.clone())
        .with_tokio()
        .with_tcp(tcp::Config::default()
                .nodelay(true)
                .port_reuse(true)
        , noise::Config::new, yamux::Config::default)?
        .with_quic()
        .with_other_transport(|k| {
            let nc = noise::Config::new(k).unwrap();
            let yc: libp2p::yamux::Config = libp2p::yamux::Config::default();
            let tc = tcp::Config::default()
                .port_reuse(false)
                .nodelay(true);
            tcp::tokio::Transport::new(tc)
                .upgrade(Version::V1Lazy)
                .authenticate(nc)
                .multiplex(yc)
        })?
        .with_other_transport(|k| {
            tcp::tokio::Transport::new(tcp::Config::default()
                .port_reuse(false)
                .nodelay(true)
            )
                .upgrade(Version::V1)
                .authenticate(noise::Config::new(k).unwrap())
                .multiplex(yamux::Config::default())
        })?
        .with_dns()?
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(|k, relay_behaviour| chain::Behaviour::from(k.clone()))?
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
