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
    Client, ClientConfig, EventEmitter, RoomState, SyncSettings,
};

#[derive(Clap)]
#[clap(version = "0.1", author = "Antoine Martin")]
struct Opts {
    /// Username to use for bot account
    #[clap(long)]
    username: String,

    /// Password for bot account
    #[clap(long)]
    password: String,

    /// Homeserver to connect to
    #[clap(long)]
    homeserver: String,

    /// Folder to store the client state into
    #[clap(long, parse(from_os_str))]
    store_path: PathBuf,
}

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

async fn login_and_sync(
    homeserver_url: String,
    username: &str,
    password: &str,
    store_path: PathBuf,
) -> Result<(), matrix_sdk::Error> {
    let client_config = ClientConfig::new().store_path(store_path);

    let homeserver_url = Url::parse(&homeserver_url).expect("Couldn't parse the homeserver URL");
    let client = Client::new_with_config(homeserver_url, client_config).unwrap();

    client
        .login(username, password, None, Some("autojoin bot"))
        .await?;

    println!("logged in as {}", username);

    client
        .add_event_emitter(Box::new(AutoJoinBot::new(client.clone())))
        .await;

    client.sync(SyncSettings::default()).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), matrix_sdk::Error> {
    tracing_subscriber::fmt::init();
    let opts = Opts::parse();

    login_and_sync(
        opts.homeserver,
        &opts.username,
        &opts.password,
        opts.store_path,
    )
    .await
}