use matrix_sdk::identifiers::RoomId;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;
use url::Url;

/// Holds the configuration for the bot.
#[derive(Clone, Deserialize)]
pub struct Config {
    /// The URL for the homeserver we should connect to
    pub homeserver: Url,
    /// The bot's account username
    pub username: String,
    /// The bot's account password
    pub password: String,
    /// Path to a directory where the bot will store Matrix state and current session information.
    pub state_dir: PathBuf,
    /// ID of the Matrix room where the bot should post messages. The bot will only accept
    /// invitations to this room.
    pub room_id: RoomId,
    /// Units to watch for logs
    pub units: HashSet<String>,
}
