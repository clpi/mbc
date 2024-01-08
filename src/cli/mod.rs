pub mod cmd;

use crate::models;

use self::{
    cmd::Cmd,
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
    pub command: Option<cmd::Cmd>,
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
impl Opts {
    pub async fn run(&self) -> anyhow::Result<()> {
        match self.command {
            Some(ref cmd) => cmd.run().await,
            None => {
                match self.mode {
                    Some(Mode::Dial) => {
                        return models::run().await;
                    },
                    Some(Mode::Listen) => {
                        return models::run().await;
                    },
                    None => {
                        println!("No mode specified");
                        return models::run().await;
                    }
                }
            }
        }
    }
}