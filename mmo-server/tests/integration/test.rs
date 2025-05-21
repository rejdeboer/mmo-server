use crate::helpers::spawn_app;
use bevy::prelude::*;
use bevy_renet::renet::DefaultChannel;
use mmo_server::server::EnterGameRequest;

#[test]
fn testing() {
    let app = spawn_app();
    let (mut client, mut transport) = app.create_client();
    let character_id = app.test_character_id;

    let is_connected = |world: &mut World, elapsed: std::time::Duration| -> bool {
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

    let response_received = |world: &mut World| -> bool { client.is_connected() };
}
