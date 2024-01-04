pub mod client;
pub mod data;
pub mod store;
pub mod server;

 pub use self::{
    client::Client,
    data::*,
    store::Store,
    server::Server,
 };

use std::{
    net::{Ipv4Addr, Ipv6Addr},
    error::Error as SE,
    collections::HashSet
};

use libp2p::{
    mdns::{self,}, noise, tcp, yamux,
    core::{Multiaddr, multiaddr::Protocol},
    identify,
    identity,
    relay,
    ping,
    kad::{
        self, Mode,
        store::MemoryStore,
    },
    swarm::{self, NetworkBehaviour, SwarmEvent},
};

use super::AppBehavior;

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
