pub struct Server;
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
    identify,
    identity::{self, Keypair as Keypair},
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

use crate::models::{BehaviourEvent, PEER_ID};

use self::{super::resp::ChainResponse, super::req::LocalChainReq};

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

use super::{init_swarm, chain};


pub async fn listen(mut swarm: libp2p::swarm::Swarm<chain::Behaviour>) {
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
                BehaviourEvent::ReqResp(req_resp) => match req_resp {
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
                BehaviourEvent::Kad(ke) => match ke {
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
                BehaviourEvent::Ping(libp2p::ping::Event { peer, connection, result }) => {
                    tracing::info!("Ping result [{connection:?}] from {:?}: {:?}", peer, result);
                },
                BehaviourEvent::Dcutr(libp2p::dcutr::Event { remote_peer_id, result }) => {
                    tracing::info!("Request from {:?}: {:?}", remote_peer_id, result);
                },
                BehaviourEvent::Fs(libp2p::floodsub::FloodsubEvent::Message(message)) => {
                    tracing::info!("Message from {:?}: {:?}", message.source, String::from_utf8_lossy(&message.data));
                },
                BehaviourEvent::Identify(ie) => match ie {
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
                BehaviourEvent::Gs(gse) => match gse {
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
                BehaviourEvent::Mdns(me) => match me {
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