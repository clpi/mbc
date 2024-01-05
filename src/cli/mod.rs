pub mod cmd;

use self::{
    cmd::Subcmd,
};

use std::{str::FromStr, path::PathBuf};

use clap::Parser;
use libp2p::{Multiaddr, PeerId};

#[derive(Debug, Clone, Parser)]
#[clap(
    name = "mbc",
    about = "A simple command line tool for interacting with the mbc network",
    version = "0.1.0",
    propagate_version = true,
    disable_help_subcommand = false,
)]
pub struct Opts {
    /// The mode (client-listen, client-dial)
    #[clap(long)]
    pub mode: Option<Mode>,

    /// Output JSON
    #[clap(long, short, global = true)]
    pub json: bool,

    /// Set verbosity
    #[clap(short, global = true, action = clap::ArgAction::Count)]
    pub verbosity: u8,

    /// Peer ID of the remote peer to hole punch to
    #[clap(long)]
    peer: Option<Multiaddr>,

    /// Fixed val to gen deterministic peer id
    #[clap(long)]
    pub key_seed: Option<u8>,

    /// Listening addr
    #[clap(long)]
    pub relay_addr: Option<Multiaddr>,

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

#[derive(clap::Subcommand, Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Subcommand {
    #[clap(arg_required_else_help = false)]
    Run {

    },
    #[clap(subcommand, arg_required_else_help = true)]
    Topics(AddCmd),
    #[clap(subcommand, arg_required_else_help = true)]
    Add(AddCmd),
    #[clap(subcommand, arg_required_else_help = true)]
    List(ListCmd),
    #[clap(subcommand, arg_required_else_help = true)]
    Config(ConfigCmd),
    /// Generate completions
    Completions {
        shell: clap_complete::Shell,
    },
    Provide {
        #[clap(long)]
        path: PathBuf,

        #[clap(long)]
        name: String,
    },
    Get {
        #[clap(long)]
        name: String
    },
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
#[derive(Clone, Debug, PartialEq, Parser)]
pub enum AddCmd {
    Artist,
    Album,
    Listen,
    Event,
    Song,
}
#[derive(Clone, Debug, PartialEq, Parser)]
pub enum ConfigCmd {
    Artist,
    Album,
    Listen,
    Event,
    Song,

}
#[derive(Clone, Debug, PartialEq, Parser)]
pub enum ListCmd {
    Artist,
    Album,
    Listen,
    Event,
    Song,

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