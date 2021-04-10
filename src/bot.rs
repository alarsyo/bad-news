use std::{
    fs::File,
    io::{BufReader, BufWriter},
};

use matrix_sdk::{
    events::{
        room::message::{MessageEventContent, TextMessageEventContent},
        AnyMessageEventContent,
    },
    Client, ClientConfig, Session, SyncSettings,
};
use systemd::{journal, JournalRecord};

use crate::autojoin::AutoJoinHandler;
use crate::Config;

#[derive(Clone)]
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
        let clone = self.clone();

        tokio::task::spawn_blocking(move || clone.watch_journald());

        self.client.sync(SyncSettings::default()).await
    }

    fn watch_journald(&self) {
        let mut reader = journal::OpenOptions::default()
            .system(true)
            .open()
            .expect("Could not open journal");

        // Seek to end of current log to prevent old messages from being printed
        reader
            .seek_tail()
            .expect("Could not seek to end of journal");

        // HACK: for some reason calling `seek_tail` above still leaves old entries when calling
        // next, so skip all those before we start the real logging
        loop {
            if reader.next().unwrap() == 0 {
                break;
            }
        }

        // NOTE: Ugly double loop, but low level `wait` has to be used if we don't want to miss any
        // new entry. See https://github.com/jmesmon/rust-systemd/issues/66
        loop {
            loop {
                let record = reader.next_entry().unwrap();
                match record {
                    Some(record) => self.handle_record(record),
                    None => break,
                }
            }

            reader.wait(None).unwrap();
        }
    }

    fn handle_record(&self, record: JournalRecord) {
        const KEY_UNIT: &str = "_SYSTEMD_UNIT";
        const KEY_MESSAGE: &str = "MESSAGE";

        if let Some(unit) = record.get(KEY_UNIT) {
            let unit_config = match self.config.units.iter().find(|u| &u.name == unit) {
                Some(config) => config,
                None => return,
            };

            let message = record.get(KEY_MESSAGE);
            if let Some(filter) = &unit_config.filter {
                if message.is_none() || !filter.is_match(message.unwrap()) {
                    return;
                }
            }

            let message = format!(
                "[{}] {}",
                unit.strip_suffix(".service").unwrap_or(unit),
                message.map(|m| m.as_ref()).unwrap_or("<EMPTY MESSAGE>")
            );
            let content = AnyMessageEventContent::RoomMessage(MessageEventContent::Text(
                TextMessageEventContent::plain(message),
            ));
            let room_id = self.config.room_id.clone();
            let client_clone = self.client.clone();

            tokio::spawn(async move {
                client_clone
                    .room_send(&room_id, content, None)
                    .await
                    .unwrap();
            });
        }
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
