use std::{
    fs::File,
    io::{BufReader, BufWriter},
};

use matrix_sdk::{Client, ClientConfig, Session, SyncSettings};

use crate::autojoin::AutoJoinHandler;
use crate::Config;

pub struct BadNewsBot {
    client: Client,
    config: Config,
}

impl BadNewsBot {
    /// Creates a new [`BadNewsBot`] and builds a [`matrix_sdk::Client`] using the provided
    /// [`Config`].
    ///
    /// The [`Client`] is only initialized, not ready to be used yet.
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let client_config = ClientConfig::new().store_path(config.state_dir.join("store"));
        let client = Client::new_with_config(config.homeserver.clone(), client_config)?;

        Ok(Self { client, config })
    }

    /// Loads session information from file, or creates it if no previous session is found.
    ///
    /// The bot is ready to run once this function has been called.
    pub async fn init(&self) -> anyhow::Result<()> {
        load_or_init_session(&self).await?;

        self.client
            .add_event_emitter(Box::new(AutoJoinHandler::new(
                self.client.clone(),
                self.config.room_id.clone(),
            )))
            .await;

        Ok(())
    }

    /// Start listening to Matrix events.
    ///
    /// [`BadNewsBot::init`] **must** be called before this function, otherwise the [`Client`] isn't
    /// logged in.
    pub async fn run(&self) {
        self.client.sync(SyncSettings::default()).await
    }
}

/// This loads the session information from an existing file, and tries to login with it. If no such
/// file is found, then login using username and password, and save the new session information on
/// disk.
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
