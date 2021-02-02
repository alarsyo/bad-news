use std::time::Duration;

use matrix_sdk::{
    self, async_trait,
    events::{room::member::MemberEventContent, StrippedStateEvent},
    Client, EventEmitter, RoomState,
};
use tokio::time::sleep;

pub struct AutoJoinHandler {
    client: Client,
}

impl AutoJoinHandler {
    pub fn new(client: Client) -> Self {
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
