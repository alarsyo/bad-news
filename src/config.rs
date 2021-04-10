use matrix_sdk::identifiers::RoomId;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::str::FromStr;
use url::Url;
use void::Void;

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
    pub units: Vec<Unit>,
}

/// Holds a single unit's configuration.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(from = "SerializedUnit")]
pub struct Unit {
    /// Can be serialized from a string only instead of a map.
    pub name: String,
    /// Regex to filter each line read from the unit's logs.
    pub filter: Option<String>, // FIXME: regex
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
struct SerializedUnit(#[serde(deserialize_with = "unit_name_or_struct")] Unit);

impl From<SerializedUnit> for Unit {
    fn from(s: SerializedUnit) -> Self {
        s.0
    }
}

impl FromStr for Unit {
    type Err = Void;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Unit {
            name: s.to_string(),
            filter: None,
        })
    }
}

fn unit_name_or_struct<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = Void>,
    D: Deserializer<'de>,
{
    struct StringOrStruct<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = Void>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            Ok(FromStr::from_str(value).unwrap())
        }

        fn visit_map<M>(self, map: M) -> Result<T, M::Error>
        where
            M: MapAccess<'de>,
        {
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(StringOrStruct(PhantomData))
}
