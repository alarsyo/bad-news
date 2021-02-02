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
