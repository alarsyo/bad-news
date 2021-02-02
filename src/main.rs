use std::{
    fs::File,
    io::{BufReader, BufWriter},
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

    /// File where session information will be saved
    #[clap(long, parse(from_os_str))]
    session: PathBuf,
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
) {
    if session_file.is_file() {
        let reader = BufReader::new(File::open(session_file).unwrap());

        let session: Session = serde_json::from_reader(reader).unwrap();

        client.restore_login(session.clone()).await.unwrap();

        println!("Reused session: {}, {}", session.user_id, session.device_id);
    } else {
        let response = client
            .login(username, password, None, Some("autojoin bot"))
            .await
            .unwrap();

        println!("logged in as {}", username);

        let session = Session {
            access_token: response.access_token,
            user_id: response.user_id,
            device_id: response.device_id,
        };

        let writer = BufWriter::new(File::create(session_file).unwrap());
        serde_json::to_writer(writer, &session).unwrap();
    }
}

async fn login_and_sync(
    homeserver_url: String,
    username: &str,
    password: &str,
    store_path: PathBuf,
    session_file: PathBuf,
) -> Result<(), matrix_sdk::Error> {
    let client_config = ClientConfig::new().store_path(store_path);

    let homeserver_url = Url::parse(&homeserver_url).expect("Couldn't parse the homeserver URL");
    let client = Client::new_with_config(homeserver_url, client_config).unwrap();

    load_or_init_session(&client, session_file, username, password).await;

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
        opts.session,
    )
    .await
}
