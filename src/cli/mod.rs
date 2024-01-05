pub mod cmd;

use self::{
    cmd::Subcmd,
};

use std::str::FromStr;

use clap::Parser;
use libp2p::{Multiaddr, PeerId};

#[derive(Debug, Parser)]
#[clap(name = "mbc")]
pub struct Opts {
    /// The mode (client-listen, client-dial)
    #[clap(long)]
    pub mode: Option<Mode>,

    /// Fixed val to gen deterministic peer id
    #[clap(long)]
    pub key_seed: Option<u8>,

    /// Listening addr
    #[clap(long)]
    pub relay_addr: Option<Multiaddr>,

    /// Peer ID of the remote peer to hole punch to
    #[clap(long)]
    pub remote_peer_id: Option<PeerId>,

    /// Have relay listen on ipv6 or ipv4 (default) loopback address
    #[clap(long)]
    pub use_ipv6: Option<bool>,

    /// Port to listen on all interfaces
    #[clap(long)]
    pub port: Option<u16>,

    /// Subcommand
    #[clap(subcommand)]
    pub command: Option<Subcommand>,
}

#[derive(Parser,Debug)]
pub enum Subcommand {
    Peers {
        #[clap(long)]
        peer_id: Option<PeerId>,
    },
    PutPkRecord {},
}

#[derive(Clone, Debug, PartialEq, Parser)]
pub enum Mode {
    Dial,
    Listen,
}

impl FromStr for Mode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "d" | "dial" => Ok(Mode::Dial),
            "l" | "listen" => Ok(Mode::Listen),
            _ => Err("Expected either dial or listen".to_string())
        }
    }
}