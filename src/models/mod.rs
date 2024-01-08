pub mod p2p;

pub use self::{
    p2p::{
        *
    },
};

use std::{
    collections::{hash_map::{self, DefaultHasher}, HashMap, HashSet},
    hash::{Hash, Hasher},
    time::{Duration, Instant}, f32::consts::E
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
    kad::{self, Mode},
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

use self::{resp::ChainResponse, req::LocalChainReq};

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

pub async fn listen(mut swarm: libp2p::swarm::Swarm<chain::Behaviour>) {
    let (mut learned_observ_addr, mut told_relay_observ_addr) = (false, false);
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
                        match result {
                            kad::QueryResult::GetProviders(prov) => match prov {
                                Ok(kad::GetProvidersOk::FoundProviders { key, providers }) => {
                                    for peer in providers {
                                        tracing::info!(
                                            "Peer {peer:?} provides key {:?}",
                                            std::str::from_utf8(key.as_ref()).unwrap()
                                        );
                                    }
                                },
                                Ok(kad::GetProvidersOk::FinishedWithNoAdditionalRecord { closest_peers }) => {
                                    tracing::info!("Finished with no additional record: {:?}", closest_peers);
                                },
                                Err(e) => {
                                    tracing::error!("Error: {:?}", e);
                                },
                            },
                            kad::QueryResult::Bootstrap(boot) => match boot {
                                Ok(b) => {
                                    tracing::info!("Finished");
                                },
                                Err(e) => {
                                    tracing::error!("Error: {:?}", e);
                                }
                            },
                            kad::QueryResult::GetRecord(rec) => match rec {
                                Ok(kad::GetRecordOk::FinishedWithNoAdditionalRecord { cache_candidates }) => {
                                    tracing::info!("Found record: {:?} ", cache_candidates);
                                },
                                Ok(kad::GetRecordOk::FoundRecord(
                                    kad::PeerRecord {  peer, record })
                                ) => match (peer, record) {
                                    (Some(p), kad::Record { key, value, publisher, expires  }) => {
                                        tracing::info!("Found record: {:?} {:?} ", key, value);
                                    },
                                    (None, record) => {
                                        tracing::info!("Found record: {:?} ", record);
                                    },
                                },
                                Err(e) => {
                                    tracing::error!("Error: {:?}", e);
                                },
                            },
                            kad::QueryResult::GetClosestPeers(closest) => match closest {
                                Ok(kad::GetClosestPeersOk { peers, key } ) => {
                                    tracing::info!("Found closest peers: {:?} {:?} ", key, peers);
                                },
                                Err(e) => {
                                    tracing::error!("Error: {:?}", e);
                                },
                            },
                            kad::QueryResult::PutRecord(put) => match put {
                                kad::PutRecordResult::Ok(r) => {
                                    tracing::info!("Put record: {:?}", r);
                                },
                                kad::PutRecordResult::Err(e) => {
                                    tracing::error!("Error: {:?}", e);
                                },
                            },
                            kad::QueryResult::StartProviding(prov) => match prov {
                                Ok(k) => {
                                    tracing::info!("Start providing: {:?}", k);
                                },
                                Err(e) => {
                                    tracing::error!("Error: {:?}", e);
                                },
                            },
                            kad::QueryResult::RepublishProvider(prov) => match prov {
                                Ok(k) => {
                                    tracing::info!("Republish provider: {:?}", k);
                                },
                                Err(e) => {
                                    tracing::error!("Error: {:?}", e);
                                },
                            },
                            kad::QueryResult::RepublishRecord(rec) => match rec {
                                Ok(k) => {
                                    tracing::info!("Republish record: {:?}", k);
                                },
                                Err(e) => {
                                    tracing::error!("Error: {:?}", e);
                                },
                            },
                        }
                    },
                    kad::Event::UnroutablePeer { peer } => {
                        tracing::warn!("Unroutable peer: {:?}", peer);
                    },
                    kad::Event::RoutablePeer { peer, address } => {
                        tracing::info!("Routable peer: {:?} at {address:?}", peer);
                    },
                },
                BehaviourEvent::Relay(re) => match re {
                    libp2p::relay::Event::CircuitClosed { src_peer_id, dst_peer_id, error } => {
                        tracing::info!("Circuit closed: {:?} {:?} {:?}", src_peer_id, dst_peer_id, error);
                    },
                    libp2p::relay::Event::CircuitReqDenied { src_peer_id, dst_peer_id } => {
                        tracing::info!("Circuit request denied: {:?} {:?}", src_peer_id, dst_peer_id);
                    },
                    libp2p::relay::Event::CircuitReqAccepted { src_peer_id, dst_peer_id } => {
                        tracing::info!("Circuit request accepted: {:?} {:?}", src_peer_id, dst_peer_id);
                    },
                    libp2p::relay::Event::ReservationReqDenied { src_peer_id } => {
                        tracing::info!("Reservation request denied: {:?}", src_peer_id);
                    },
                    libp2p::relay::Event::ReservationReqAccepted { src_peer_id, renewed } => {
                        tracing::info!("Reservation request accepted: {:?}", src_peer_id);
                    },
                    libp2p::relay::Event::ReservationTimedOut { src_peer_id } => {
                        tracing::info!("Reservation timed out: {:?}", src_peer_id);
                    },
                    _ => {
                        tracing::info!("Unhandled relay event");
                        panic!("Unhandled relay event");
                    },
                },
                BehaviourEvent::Ping(libp2p::ping::Event {peer, connection, result}) => match result {
                    Ok(r) => {
                        tracing::info!("Ping result [{connection:?}] from {:?}: {:?}", peer, r);
                    }
                    Err(libp2p::ping::Failure::Timeout) => {
                        tracing::info!("Ping result [{connection:?}] from {:?}: {:?}", peer, libp2p::ping::Failure::Timeout);
                    }
                    Err(libp2p::ping::Failure::Unsupported) => {
                        tracing::info!("Ping result [{connection:?}] from {:?}: {:?}", peer, libp2p::ping::Failure::Unsupported);
                    }
                    Err(libp2p::ping::Failure::Other { error }) => {
                        tracing::info!("Ping result [{connection:?}] from {:?}: {:?}", peer, libp2p::ping::Failure::Other { error });
                    }
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
                    identify::Event::Pushed { peer_id, info: identify::Info {
                        observed_addr, 
                        public_key, 
                        protocol_version, 
                        agent_version, 
                        listen_addrs, 
                        protocols 
                    } } => {
                        for p in protocols {
                            tracing::info!("Protocol to {p:?}");
                        }
                        for a in listen_addrs {
                            tracing::info!("Sent to {a:?}");
                        }
                        tracing::info!("Pushed to {peer_id:?}");
                    },
                    identify::Event::Sent { peer_id } => {
                        told_relay_observ_addr = true;
                        tracing::info!("Sent to {peer_id:?}");
                    },
                    identify::Event::Received { info: identify::Info { 
                        observed_addr, 
                        public_key, 
                        protocol_version, 
                        agent_version, 
                        listen_addrs, 
                        protocols 
                    }, peer_id } => {
                        swarm.add_external_address(observed_addr.clone());
                        for p in protocols {
                            tracing::info!("Protocol to {p:?}");
                        }
                        for a in listen_addrs {
                            tracing::info!("Sent to {a:?}");
                        }
                        tracing::info!("Received from {peer_id:?}");
                        tracing::info!(address=%observed_addr, "Relay told us our observed address");
                        learned_observ_addr = true;
                    },
                },

                BehaviourEvent::Rs(rse) => match rse {
                    libp2p::rendezvous::server::Event::DiscoverNotServed { enquirer, error } => {
                        tracing::warn!("Discover not served: {:?} {:?}", enquirer, error);
                    },
                    libp2p::rendezvous::server::Event::PeerRegistered { peer, registration } => {
                        tracing::info!("Peer registered: {:?} {:?}", peer, registration);
                    },
                    libp2p::rendezvous::server::Event::PeerNotRegistered { peer, namespace, error } => {
                        tracing::info!("Peer unregistered: {:?} {:?}", peer, error);
                    },
                    libp2p::rendezvous::server::Event::RegistrationExpired(reg) => {
                        tracing::info!("Registration expired: {:?}", reg);
                    },
                    libp2p::rendezvous::server::Event::DiscoverServed { enquirer, registrations } => {
                        tracing::info!("Discover served: {:?} {:?}", enquirer, registrations);
                    },
                    _ => {
                        tracing::info!("Unhandled rendezvous server event");
                        panic!("Unhandled rendezvous server event");
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
                BehaviourEvent::An(ae) => match ae {
                    libp2p::autonat::Event::InboundProbe(_probe) => {
                        tracing::info!("Inbound probe");
                    },
                    libp2p::autonat::Event::OutboundProbe(_probe) => {
                        tracing::info!("Outbound probe");
                    },
                    libp2p::autonat::Event::StatusChanged { old, new } => {
                        tracing::info!("Status changed: {:?} -> {:?}", old, new);
                    },
                },
                BehaviourEvent::Upnp(upe) => match upe {
                    libp2p::upnp::Event::ExpiredExternalAddr(_addr) => {
                        tracing::info!("Expired external address");
                    },
                    libp2p::upnp::Event::NewExternalAddr(_addr) => {
                        tracing::info!("New external address");
                    },
                    libp2p::upnp::Event::GatewayNotFound => {
                        tracing::info!("Gateway not found");
                    },
                    libp2p::upnp::Event::NonRoutableGateway => {
                        tracing::info!("Non routable gateway");
                    },
                },
                BehaviourEvent::Mdns(me) => match me {
                    libp2p::mdns::Event::Discovered(list) => {
                        for (peer_id, _multiaddr) in list {
                            tracing::info!("mDNS discovered a new peer: {peer_id}");
                            swarm.behaviour_mut().gs.add_explicit_peer(&peer_id);
                        }
                    },
                    libp2p::mdns::Event::Expired(list) => {
                        for (peer_id, _multiaddr) in list {
                            tracing::info!("mDNS discover peer has expired: {peer_id}");
                            swarm.behaviour_mut().gs.remove_explicit_peer(&peer_id);
                        }
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
        if learned_observ_addr && told_relay_observ_addr { 
            tracing::info!("learned_observ_addr & told_relay_observ_addr");
            break; 
        }
    }

}
pub async fn run() -> anyhow::Result<()> {
    let mut swarm = init_swarm().await.unwrap();
    swarm.behaviour_mut().kad.set_mode(Some(Mode::Server));
    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    // swarm.dial(opts.relay_addr.clone()?);
    // for peer in &BOOTNODES {
    //     swarm.behaviour_mut()
    //         .add_address(&peer.parse()?, "/dnsaddr/bootstrap.libp2p.io".parse()?);
    // }
    block_on(listen(swarm));
    Ok(())
}