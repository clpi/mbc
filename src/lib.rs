#![doc = include_str!("../README.md")]

use clap::{Parser};
use cli::Subcommand;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

pub mod models;
pub mod cli;
pub mod store;

fn init_log() -> () {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_line_number(true)
        .compact()
        // .pretty()
        .with_ansi(true)
        .with_max_level(LevelFilter::DEBUG)
        .try_init()
        .unwrap_or_else(|e| {
            eprintln!("Failed to init log: {:?}", e);
        })
}

fn init_opts() -> anyhow::Result<cli::Opts> {
    let opts = cli::Opts::parse();
    Ok(opts)
}


pub async fn init() -> anyhow::Result<()> {
    init_log();
    let opts = init_opts().unwrap();
    match opts.command {
        Some(Subcommand::Peers { peer_id }) => {
            let peers = models::p2p::get_list_peers();
            tracing::info!("Peers: {:?}", peers);
        },
        Some(Subcommand::PutPkRecord { .. }) => {
            let chain = models::p2p::get_chain();
            tracing::info!("Chain: {:?}", chain);
        },
        None => {
            tracing::info!("No subcommand");
        }
    }
    models::p2p::run().await
}
