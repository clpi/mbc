#![doc = include_str!("../README.md")]

use clap::{Parser};
use cli::{cmd::{Cmd, self}, Opts};

pub mod models;
pub mod cli;
pub mod store;


pub async fn init() -> anyhow::Result<()> {
    Opts::parse().run().await
}
