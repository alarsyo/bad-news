use std::time::Duration;

use matrix_sdk::{
    self, async_trait,
    events::{room::member::MemberEventContent, StrippedStateEvent},
    identifiers::RoomId,
    room::Room,
    Client, EventHandler,
};
use tokio::time::sleep;

pub struct AutoJoinHandler {
    client: Client,
    room_id: RoomId,
}

impl AutoJoinHandler {
    pub fn new(client: Client, room_id: RoomId) -> Self {
        Self { client, room_id }
    }
}

#[async_trait]
impl EventHandler for AutoJoinHandler {
    async fn on_stripped_state_member(
        &self,
        room: Room,
        room_member: &StrippedStateEvent<MemberEventContent>,
        _: Option<MemberEventContent>,
    ) {
        if room_member.state_key != self.client.user_id().await.unwrap() {
            return;
        }

        if let Room::Invited(room) = room {
            let room_id = room.room_id();
            let room_name = room
                .display_name()
                .await
                .expect("couldn't get joined room name!");
            println!(
                "Received invitation for room `{}`: `{}`",
                room_id, room_name
            );

            if room_id != &self.room_id {
                println!(
                    "Bot isn't authorized to join room `{}`, declining invitation",
                    room_id
                );
                room.reject_invitation().await.unwrap();
                return;
            }

            println!("Autojoining room {}", room.room_id());
            let mut delay = 2;

            while let Err(err) = room.accept_invitation().await {
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
