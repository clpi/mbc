use std::str::FromStr;

use clap::Parser;
use libp2p::{Multiaddr, PeerId};

#[derive(Debug, Parser)]
#[clap(name = "libp2p dcutr")]
pub struct Opts {
    /// The mode (client-listen, client-dial)
    #[clap(long)]
    mode: Mode,

    /// Fixed val to gen deterministic peer id
    #[clap(long)]
    key_seed: u8,

    /// Listening addr
    #[clap(long)]
    relay_addr: Multiaddr,

    /// Peer ID of the remote peer to hole punch to
    #[clap(long)]
    remote_peer_id: Option<PeerId>,

    #[clap(subcommand)]
    arg: CliArgument,
}

#[derive(Parser,Debug)]
pub enum CliArgument {
    GetPeers {
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