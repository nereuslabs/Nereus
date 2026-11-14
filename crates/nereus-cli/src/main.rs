//! Nereus 2.0 — The Terminal-First AI Programming Mentor Built in Rust
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

#![deny(warnings)]

use anyhow::Result;
use clap::Parser;
use nereus_cli::{run, Cli};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Cli::parse();
    run(args).await
}
