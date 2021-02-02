use std::{
    fs::File,
    io::{self, BufReader, BufWriter},
};
use std::{path::PathBuf, time::Duration};

use clap::Clap;

use tokio::time::sleep;
use url::Url;

use matrix_sdk::{
    self, async_trait,
    events::{
        room::{
            member::MemberEventContent,
            message::{MessageEventContent, TextMessageEventContent},
        },
        StrippedStateEvent, SyncMessageEvent,
    },
    Client, ClientConfig, EventEmitter, RoomState, Session, SyncSettings,
};

use serde::Deserialize;

use thiserror::Error;

struct AutoJoinBot {
    client: Client,
}

impl AutoJoinBot {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl EventEmitter for AutoJoinBot {
    async fn on_room_message(
        &self,
        room: RoomState,
        event: &SyncMessageEvent<MessageEventContent>,
    ) {
        if let RoomState::Joined(room) = room {
            if let SyncMessageEvent {
                content: MessageEventContent::Text(TextMessageEventContent { body: msg_body, .. }),
                sender,
                ..
            } = event
            {
                let member = room.get_member(&sender).await.unwrap().unwrap();
                let name = member
                    .display_name()
                    .unwrap_or_else(|| member.user_id().as_str());
                println!("{}: {}", name, msg_body);
            }
        }
    }
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

// TODO: use nice error handling
async fn load_or_init_session(
    client: &Client,
    session_file: PathBuf,
    username: &str,
    password: &str,
) -> anyhow::Result<()> {
    if session_file.is_file() {
        let reader = BufReader::new(File::open(session_file)?);

        let session: Session = serde_yaml::from_reader(reader)?;

        client.restore_login(session.clone()).await?;

        println!("Reused session: {}, {}", session.user_id, session.device_id);
    } else {
        let response = client
            .login(username, password, None, Some("autojoin bot"))
            .await?;

        println!("logged in as {}", username);

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

async fn login_and_sync(
    homeserver_url: Url,
    username: &str,
    password: &str,
    state_dir: PathBuf,
) -> anyhow::Result<()> {
    let client_config = ClientConfig::new().store_path(state_dir.join("store"));

    let client = Client::new_with_config(homeserver_url, client_config)?;

    load_or_init_session(&client, state_dir.join("session.yaml"), username, password).await?;

    client
        .add_event_emitter(Box::new(AutoJoinBot::new(client.clone())))
        .await;

    client.sync(SyncSettings::default()).await;

    Ok(())
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

#[derive(Deserialize)]
struct Config {
    homeserver: Url,
    username: String,
    password: String,
    state_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let opts = Opts::parse();
    let config_file = opts.config;

    let config: Config = serde_yaml::from_reader(BufReader::new(File::open(config_file)?))?;

    login_and_sync(
        config.homeserver,
        &config.username,
        &config.password,
        config.state_dir,
    )
    .await
}
