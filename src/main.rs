use std::{
    fs::File,
    io::{self, BufReader},
    path::PathBuf,
};

use clap::Clap;
use matrix_sdk::identifiers::RoomId;
use serde::Deserialize;
use thiserror::Error;
use url::Url;

mod autojoin;
mod bot;

use bot::BadNewsBot;

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

/// Holds the configuration for the bot.
#[derive(Deserialize)]
pub struct Config {
    /// The URL for the homeserver we should connect to
    homeserver: Url,
    /// The bot's account username
    username: String,
    /// The bot's account password
    password: String,
    /// Path to a directory where the bot will store Matrix state and current session information.
    state_dir: PathBuf,
    /// ID of the Matrix room where the bot should post messages. The bot will only accept
    /// invitations to this room.
    room_id: RoomId,
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
