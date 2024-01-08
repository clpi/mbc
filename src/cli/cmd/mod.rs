use std::path::PathBuf;

use clap::{Parser, Subcommand};
use libp2p::PeerId;

#[derive(clap::Subcommand, Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Cmd {
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
    Init,
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
pub enum InitCmd {

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

impl Cmd {
    pub async fn run(&self) -> anyhow::Result<()> {
        match self {
            Cmd::Run {..} => {
                tracing::info!("Run");
            },
            Cmd::Add {..} => {
                tracing::info!("Add");
            },
            Cmd::List {..} => {
                tracing::info!("List");
            },
            Cmd::Topics {..} => {
                tracing::info!("Topics");
            },
            Cmd::Provide { path, name } => {
                tracing::info!("Provide: {:?} {:?}", path, name);
            },
            Cmd::Get { name } => {
                tracing::info!("Get: {:?}", name);
            },
            Cmd::Init => {
                tracing::info!("Init");
            },
            Cmd::Config {..} => {
                tracing::info!("Config");
            },
            Cmd::Completions { shell } => {
                tracing::info!("Completions: {:?}", shell);
            },
            Cmd::Peers { peer_id } => {
                let peers = crate::models::p2p::get_list_peers();
                tracing::info!("Peers: {:?}", peers);
            },
            Cmd::PutPkRecord { .. } => {
                let chain = crate::models::p2p::get_chain();
                tracing::info!("Chain: {:?}", chain);
            },
        }
        Ok(())
    }
}