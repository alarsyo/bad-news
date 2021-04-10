use std::{
    fs::File,
    io::{self, BufReader},
    path::PathBuf,
};

use clap::Clap;
use thiserror::Error;

mod autojoin;
mod bot;
mod config;

use bot::BadNewsBot;
use config::Config;

#[derive(Error, Debug)]
enum BadNewsError {
    #[error("problem accessing configuration file")]
    ConfigFile(#[from] io::Error),
    #[error("Matrix communication error")]
    Matrix(#[from] matrix_sdk::Error),
}

#[derive(Clap)]
#[clap(version = "0.1", author = "Antoine Martin")]
struct Opts {
    /// File where session information will be saved
    #[clap(short, long, parse(from_os_str))]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let opts = Opts::parse();
    let config_file = opts.config;

    let config: Config = serde_yaml::from_reader(BufReader::new(File::open(config_file)?))?;

    let bot = BadNewsBot::new(config)?;
    bot.init().await?;
    bot.run().await;

    Ok(())
}
