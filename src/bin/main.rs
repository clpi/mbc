use std::time::{Duration, Instant};
use clap::{Arg, Parser};
use anyhow::anyhow;
use futures::{executor::block_on, FutureExt};
use libp2p::{
    identity::{Keypair as KeyP, ed25519::Keypair},
    core::{upgrade},
    futures::StreamExt,
    // noise::{Keypair, NoiseConfig, X25519Spec},
    tcp::tokio::{Tcp, TcpStream},
    tcp::{Transport, Config},
    swarm::{SwarmEvent, NetworkBehaviour, Swarm}, floodsub::Floodsub,
};
use mbc::models::{block::Chain, PEER_ID};
use tokio::{
    sync::mpsc,
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    time::sleep,
};
use tracing_subscriber::EnvFilter;

pub const BOOTNODES: [&str; 4] = [
    "QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
    "QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
    "QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
    "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
];

#[derive(NetworkBehaviour)]
pub struct ChainBehaviour {
    // relay_client: libp2p::relay::client::Behaviour,
    ping: libp2p::ping::Behaviour,
    identify: libp2p::identify::Behaviour,
    dcutr: libp2p::dcutr::Behaviour,
    fs: libp2p::floodsub::Floodsub,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // pretty_env_logger::init();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();
    // let opts = mbc::cli::Opts::parse();
    log::info!("Peer id: {}", PEER_ID.clone());
    // let (resp_send, mut resp_recv) = mpsc::unbounded_channel();
    // let (init_send, mut init_recv) = mpsc::unbounded_channel();
    let auth_keys = KeyP::generate_ed25519();
    let ak = auth_keys.public().to_peer_id();
    let transp = Config::new();
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(auth_keys.clone())
        .with_tokio()
        .with_tcp(
            Config::default()
                .port_reuse(true)
                .nodelay(true),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_dns()?
        .with_behaviour(|k| {
            let mut cfg = libp2p::kad::Config::default();
            cfg.set_query_timeout(Duration::from_secs(5 * 60));
            let store = libp2p::kad::store::MemoryStore::new(ak,);
            ChainBehaviour {
                fs: Floodsub::new(k.public().to_peer_id().clone()),
                // relay_client: relay_behaviour,
                ping: libp2p::ping::Behaviour::new(libp2p::ping::Config::new()),
                identify: libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
                    "/TODO/0.0.1".to_string(),
                    k.public(),
                )),
                dcutr: libp2p::dcutr::Behaviour::new(k.public().to_peer_id())
            }
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(5)))
        .build();
    println!("Auth keys: {:?}", ak);
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    block_on(async {
        let mut delay = futures_timer::Delay::new(
            std::time::Duration::from_secs(1)).fuse()
        ;
        loop {
            futures::select! {
                event = swarm.next() => {
                    match event.unwrap() {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            tracing::info!(%address, "Listening on");
                        }
                        event => panic!("{event:?}"),
                    }
                }
                _ = delay => { break; }
            }
        }
    });
    block_on(async {
        let mut learned_observ_addr = false;
        let mut told_relay_observ_addr = false;
        loop {
            match swarm.next().await.unwrap() {
                SwarmEvent::NewListenAddr { listener_id, address } => {},
                SwarmEvent::Dialing { .. } => {},
                SwarmEvent::ConnectionEstablished { peer_id, connection_id, endpoint, num_established, concurrent_dial_errors, established_in } => { },
                SwarmEvent::Behaviour(b) => match b {
                    _ => {},
                }
                event => panic!("{event:?}"),
                _ => {},
            }
            if learned_observ_addr && told_relay_observ_addr { break; }
        }
    });
    // swarm.dial(opts.relay_addr.clone()?);
    // for peer in &BOOTNODES {
    //     swarm.behaviour_mut()
    //         .add_address(&peer.parse()?, "/dnsaddr/bootstrap.libp2p.io".parse()?);
    // }
    Ok(())
}


pub async fn listen() {
    loop {
    }

}

pub fn gen_ed25519(seed: u8) -> libp2p::identity::Keypair {
    let mut byt = [0u8; 32];
    byt[0] = seed;
    libp2p::identity::Keypair::ed25519_from_bytes(byt)
        .expect("Failed to generate keypair wrong leng")
}