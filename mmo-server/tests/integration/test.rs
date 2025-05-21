use crate::helpers::spawn_app;
use bevy::prelude::*;
use bevy_renet::renet::DefaultChannel;
use mmo_server::server::{EnterGameRequest, EnterGameResponse};

#[test]
fn testing() {
    let mut app = spawn_app();
    let (mut client, mut transport) = app.create_client();
    let character_id = app.test_character_ids[0];

    let is_connected = |_: &mut World, elapsed: std::time::Duration| -> bool {
        client.update(elapsed);
        transport.update(elapsed, &mut client).unwrap();
        client.is_connected()
    };
    app.run_until_condition_or_timeout(is_connected).unwrap();

    let enter_game_request = EnterGameRequest {
        token: "todo".to_string(),
        character_id,
    };
    client.send_message(
        DefaultChannel::ReliableOrdered,
        bincode::encode_to_vec(enter_game_request, bincode::config::standard()).unwrap(),
    );
    transport.send_packets(&mut client).unwrap();

    let response_received = |world: &mut World, elapsed: std::time::Duration| -> bool {
        client.update(elapsed);
        transport.update(elapsed, &mut client).unwrap();
        if let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
            let (res, _) = bincode::serde::decode_from_slice::<EnterGameResponse, _>(
                &message,
                bincode::config::standard(),
            )
            .unwrap();
            assert_eq!(res.character_data.id, character_id);
            return true;
        }
        false
    };
    app.run_until_condition_or_timeout(response_received)
        .unwrap();
}
