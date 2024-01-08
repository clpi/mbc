#![doc = include_str!("../README.md")]

use clap::{Parser};
use cli::{cmd::{Cmd, self}, Opts};
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
        .with_ansi(true)
        .with_max_level(LevelFilter::INFO)
        .try_init()
        .unwrap_or_else(|e| {
            eprintln!("Failed to init log: {:?}", e);
        })
}


pub async fn init() -> anyhow::Result<()> {
    init_log();
    Opts::parse().run().await
}
