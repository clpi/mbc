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
use serde::de;
use tokio::{
    sync::mpsc,
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    time::sleep,
};
use tracing::{Level, level_filters::LevelFilter, instrument::WithSubscriber, Subscriber};
use tracing_subscriber::{EnvFilter, fmt::SubscriberBuilder};

pub type Sender<T> = oneshot::Sender<T>;
pub type SendResult<T> = Sender<Result<T, Box<dyn SError>>>;
 pub use self::{
    chain::*,
    // client::Client,
    data::*,
    store::Store,
    server::Server,
 };


#[derive(Debug)]
pub enum Command {
    StartListening {
        addr: libp2p::Multiaddr,
        send: SendResult<()>,
    },
    Dial {
        peer_id: PeerId,
        peer_addr: Multiaddr,
        send: SendResult<()>,
    },
    StartProviding {
        file_name: String,
        send: Sender<()>,
    },
    GetProviders {
        file_name: String,
        send: Sender<HashSet<PeerId>>,
    },
    RequestFile {
        file_name: String,
        peer_id: PeerId,
        send: SendResult<Vec<u8>>,
    },
    RespondFile {
        file: Vec<u8>,
        channel: ResponseChannel<ChainResponse>,
    }
}

#[derive(Debug)]
pub enum Event {
    Init,
    OutboundRequest {
        req: String,
        channel: ResponseChannel<LocalChainReq>,
    },
    InboundRequest {
        req: String,
        channel: ResponseChannel<ChainResponse>,
    }
}
#[derive(Clone)]
pub struct Client {
    send: mpsc::Sender<Command>,
    addr: libp2p::Multiaddr,
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

pub async fn init_swarm() -> Result<Swarm<ChainBehaviour>, Box<dyn SError>> {
    let auth_keys = KeyP::generate_ed25519();
    let tcp_conf = libp2p::tcp::Config::default()
        .port_reuse(true)
        .nodelay(true);
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(auth_keys.clone())
        .with_tokio()
        .with_tcp(
            tcp_conf,
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_dns()?
        .with_behaviour(|k|
            ChainBehaviour::default()
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

pub async fn listen(mut swarm: libp2p::swarm::Swarm<ChainBehaviour>) {
    let (mut learned_observ_addr, mut _told_relay_observ_addr) = (false, false);
    loop {
        let mut _delay = futures_timer::Delay::new(
            std::time::Duration::from_secs(1)).fuse();
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { listener_id, address } => {
                tracing::info!("Listening on address {address:?} to id {listener_id:?}");
                if !learned_observ_addr {
                    swarm.behaviour_mut()
                        .kad
                        .add_address(&PEER_ID, address.clone());
                    tracing::info!("Learned listener observer [{listener_id:?}] address {address:?}");
                    learned_observ_addr = true;
                }
            },
            SwarmEvent::Dialing { peer_id, connection_id } => {
                tracing::info!("Connection {connection_id:?} dialing: {:?}", peer_id);
            },
            SwarmEvent::ExternalAddrConfirmed { address } => {
                tracing::info!("External address confirmed: {:?}", address);
            },
            SwarmEvent::ListenerClosed { listener_id, addresses, reason } => {
                tracing::warn!("Listener id {listener_id:?} addresses {addresses:?} closed: {:?}", reason);
            },
            SwarmEvent::IncomingConnection { connection_id, local_addr, send_back_addr } => {
                tracing::info!("Incoming connection id {connection_id:?}: {local_addr:?} {send_back_addr:?}");
            },
            SwarmEvent::IncomingConnectionError { connection_id, local_addr, send_back_addr, error } => {
                tracing::error!("Incoming connection id {connection_id:?} error: {local_addr:?} {send_back_addr:?}");
                panic!("{:?}", error);
            },
            SwarmEvent::ExpiredListenAddr { listener_id, address } => {
                tracing::warn!("Expired listen addr: {listener_id:?} {:?}", address);
            },
            SwarmEvent::NewExternalAddrCandidate { address } => {
                tracing::info!("New external addr candidate: {:?}", address);
            },
            SwarmEvent::ExternalAddrExpired { address } => {
                tracing::warn!("External addr expired: {:?}", address);
            },
            SwarmEvent::OutgoingConnectionError { connection_id, peer_id, error } => {
                tracing::error!("Outgoing connection {connection_id:?} to {peer_id:?} error: {:?}", error);
                panic!("{:?}", error);
            },
            SwarmEvent::ListenerError { listener_id, error } => {
                tracing::warn!("Listener closed: {:?} {:?}", listener_id, error);
                panic!("{:?}", error);
            },
            SwarmEvent::ConnectionClosed { peer_id, connection_id, endpoint, num_established, cause } => {
                tracing::info!("Connection {connection_id:?} closed for {peer_id}: {:?} by {cause:?} at {endpoint:?} [rem: {num_established}]", cause);
            },
            SwarmEvent::ConnectionEstablished { endpoint, established_in, num_established, peer_id, connection_id, concurrent_dial_errors } => {
                tracing::info!("Connection {connection_id:?} established: {:?} at {endpoint:?} in {established_in:?} [rem {num_established:?} errors {concurrent_dial_errors:?}]", peer_id);
            },
            SwarmEvent::Behaviour(b) => match b {
                ChainBehaviourEvent::ReqResp(req_resp) => match req_resp {
                    request_response::Event::Message { peer, message } => {
                        tracing::info!("Message from {:?}: {:?}", peer, message);
                    },
                    request_response::Event::OutboundFailure { peer, request_id, error } => {
                        tracing::error!("Outbound failure [id {request_id:?}] to {:?}: {:?}", peer, error);
                    },
                    request_response::Event::InboundFailure { peer, request_id, error } => {
                        tracing::error!("Inbound failure [id {request_id:?}] from {:?}: {:?}", peer, error);
                    },
                    request_response::Event::ResponseSent { peer, request_id } => {
                        tracing::info!("Response sent to {:?}: {:?}", peer, request_id);
                    },
                },
                ChainBehaviourEvent::Kad(ke) => match ke {
                    kad::Event::InboundRequest { request } => {
                        tracing::info!("Inbound request: {:?}", request);
                    },
                    kad::Event::RoutingUpdated { peer, old_peer, is_new_peer, addresses, bucket_range } => {
                        tracing::info!("Routing updated [{is_new_peer}]: from {old_peer:?} to {peer:?} [{addresses:?}] [{bucket_range:?}]");
                    },
                    kad::Event::PendingRoutablePeer { peer, address  }  => {
                        tracing::debug!("Pending routable peer: {:?} at {address:?}", peer);
                    },
                    kad::Event::ModeChanged { new_mode } => {
                        tracing::info!("Mode changed: {:?}", new_mode);
                    },
                    kad::Event::OutboundQueryProgressed { id, result, stats, step } => {
                        tracing::info!("Outbound query {id:?} progressed: {:?} [{step:?}] [{stats:?}", result);
                    },
                    kad::Event::UnroutablePeer { peer } => {
                        tracing::warn!("Unroutable peer: {:?}", peer);
                    },
                    kad::Event::RoutablePeer { peer, address } => {
                        tracing::info!("Routable peer: {:?} at {address:?}", peer);
                    },
                },
                ChainBehaviourEvent::Ping(libp2p::ping::Event { peer, connection, result }) => {
                    tracing::info!("Ping result [{connection:?}] from {:?}: {:?}", peer, result);
                },
                ChainBehaviourEvent::Dcutr(libp2p::dcutr::Event { remote_peer_id, result }) => {
                    tracing::info!("Request from {:?}: {:?}", remote_peer_id, result);
                },
                ChainBehaviourEvent::Fs(libp2p::floodsub::FloodsubEvent::Message(message)) => {
                    tracing::info!("Message from {:?}: {:?}", message.source, String::from_utf8_lossy(&message.data));
                },
                ChainBehaviourEvent::Identify(ie) => match ie {
                    identify::Event::Error { error, peer_id } => {
                        tracing::error!("Error from {peer_id:?}: {error:?}");
                        panic!("{:?}", error);
                    },
                    identify::Event::Pushed { peer_id, info } => {
                        tracing::info!("Pushed {info:?} to {peer_id:?}");
                    },
                    identify::Event::Sent { peer_id } => {
                        tracing::info!("Sent to {peer_id:?}");
                    },
                    identify::Event::Received { info, peer_id } => {
                        tracing::info!("Received {info:?} from {peer_id:?}");
                    },
                },
                ChainBehaviourEvent::Gs(gse) => match gse {
                    GSEvent::GossipsubNotSupported { peer_id } => {
                        tracing::warn!("Gossipsub not supported by {peer_id:?}");
                    },
                    GSEvent::Message {propagation_source, message, message_id} => {

                        tracing::info!("Message from {:?}: {:?}", message.source, String::from_utf8_lossy(&message.data));
                    },
                    GSEvent::Subscribed { peer_id, topic } => {
                        tracing::info!("Subscribed to {:?} from {:?}", topic, peer_id);
                    },
                    GSEvent::Unsubscribed { peer_id, topic } => {
                        tracing::info!("Unsubscribed from {:?} from {:?}", topic, peer_id);
                    },
                },
                ChainBehaviourEvent::Mdns(me) => match me {
                    libp2p::mdns::Event::Discovered(list) => {
                        tracing::info!("Discovered: {:?}", list);
                    },
                    libp2p::mdns::Event::Expired(list) => {
                        tracing::info!("Expired: {:?}", list);
                    },
                },
                _ => {
                    tracing::info!("Unhandled behaviour event");
                    panic!("Unhandled chain behaviour event event");
                },
            },
            _ => {
                tracing::info!("Unhandled swarm event");
                panic!("Unhandled swarm event");
            }
        }
        if learned_observ_addr && _told_relay_observ_addr { 
            tracing::info!("learned_observ_addr & told_relay_observ_addr");
            break; 
        }
    }

}
pub async fn run() -> anyhow::Result<()> {
    let mut swarm = init_swarm().await.unwrap();
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    // swarm.dial(opts.relay_addr.clone()?);
    // for peer in &BOOTNODES {
    //     swarm.behaviour_mut()
    //         .add_address(&peer.parse()?, "/dnsaddr/bootstrap.libp2p.io".parse()?);
    // }
    block_on(listen(swarm));
    Ok(())
}
pub fn gen_ed25519(seed: u8) -> libp2p::identity::Keypair {
    let mut byt = [0u8; 32];
    byt[0] = seed;
    libp2p::identity::Keypair::ed25519_from_bytes(byt)
        .expect("Failed to generate keypair wrong leng")
}