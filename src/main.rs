use std::{
    fs::File,
    io::{self, BufReader, BufWriter},
    path::PathBuf,
    time::Duration,
};

use clap::Clap;
use matrix_sdk::{
    self, async_trait,
    events::{room::member::MemberEventContent, StrippedStateEvent},
    Client, ClientConfig, EventEmitter, RoomState, Session, SyncSettings,
};
use serde::Deserialize;
use thiserror::Error;
use tokio::time::sleep;
use url::Url;

struct BadNewsBot {
    client: Client,
    config: Config,
}

impl BadNewsBot {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let client_config = ClientConfig::new().store_path(config.state_dir.join("store"));
        let client = Client::new_with_config(config.homeserver.clone(), client_config)?;

        Ok(Self { client, config })
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        load_or_init_session(&self).await?;

        self.client
            .add_event_emitter(Box::new(AutoJoinHandler::new(self.client.clone())))
            .await;

        Ok(())
    }

    pub async fn run(&self) {
        self.client.sync(SyncSettings::default()).await
    }
}

async fn load_or_init_session(bot: &BadNewsBot) -> anyhow::Result<()> {
    let session_file = bot.config.state_dir.join("session.yaml");

    if session_file.is_file() {
        let reader = BufReader::new(File::open(session_file)?);

        let session: Session = serde_yaml::from_reader(reader)?;

        bot.client.restore_login(session.clone()).await?;

        println!("Reused session: {}, {}", session.user_id, session.device_id);
    } else {
        let response = bot
            .client
            .login(
                &bot.config.username,
                &bot.config.password,
                None,
                Some("autojoin bot"),
            )
            .await?;

        println!("logged in as {}", bot.config.username);

        let session = Session {
            access_token: response.access_token,
            user_id: response.user_id,
            device_id: response.device_id,
        };

        let writer = BufWriter::new(File::create(session_file)?);
        serde_yaml::to_writer(writer, &session)?;
    }

    Ok(())
}

struct AutoJoinHandler {
    client: Client,
}

impl AutoJoinHandler {
    fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl EventEmitter for AutoJoinHandler {
    async fn on_stripped_state_member(
        &self,
        room: RoomState,
        room_member: &StrippedStateEvent<MemberEventContent>,
        _: Option<MemberEventContent>,
    ) {
        if room_member.state_key != self.client.user_id().await.unwrap() {
            return;
        }

        if let RoomState::Invited(room) = room {
            // TODO: only join room if it's the room specified in the configuration

            println!("Autojoining room {}", room.room_id());
            let mut delay = 2;

            while let Err(err) = self.client.join_room_by_id(room.room_id()).await {
                // retry autojoin due to synapse sending invites, before the
                // invited user can join for more information see
                // https://github.com/matrix-org/synapse/issues/4345
                eprintln!(
                    "Failed to join room {} ({:?}), retrying in {}s",
                    room.room_id(),
                    err,
                    delay
                );

                sleep(Duration::from_secs(delay)).await;
                delay *= 2;

                if delay > 3600 {
                    eprintln!("Can't join room {} ({:?})", room.room_id(), err);
                    break;
                }
            }
            println!("Successfully joined room {}", room.room_id());
        }
    }
}

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
struct Config {
    /// The URL for the homeserver we should connect to
    homeserver: Url,
    /// The bot's account username
    username: String,
    /// The bot's account password
    password: String,
    /// Path to a directory where the bot will store Matrix state and current session information.
    state_dir: PathBuf,
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
