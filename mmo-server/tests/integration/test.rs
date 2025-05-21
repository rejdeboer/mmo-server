use crate::helpers::spawn_app;
use bevy::prelude::*;
use bevy_renet::renet::DefaultChannel;
use mmo_server::server::{EnterGameRequest, EnterGameResponse};

#[test]
fn testing() {
    let mut app = spawn_app(1);
    let mut client = &app.clients[0];

    let is_connected = |_: &mut World| -> bool { client.client.is_connected() };
    app.run_until_condition_or_timeout(is_connected).unwrap();

    let enter_game_request = EnterGameRequest {
        token: "todo".to_string(),
        character_id: client.character_id,
    };
    client.client.send_message(
        DefaultChannel::ReliableOrdered,
        bincode::encode_to_vec(enter_game_request, bincode::config::standard()).unwrap(),
    );

    let response_received = |world: &mut World| -> bool {
        if let Some(message) = client
            .client
            .receive_message(DefaultChannel::ReliableOrdered)
        {
            let (res, _) = bincode::serde::decode_from_slice::<EnterGameResponse, _>(
                &message,
                bincode::config::standard(),
            )
            .unwrap();
            assert_eq!(res.character_data.id, client.character_id);
            return true;
        }
        false
    };
    app.run_until_condition_or_timeout(response_received)
        .unwrap();
}
