use crate::helpers::spawn_app;
use bevy::prelude::*;
use bevy_renet::renet::DefaultChannel;
use mmo_server::server::{EnterGameRequest, EnterGameResponse};

#[test]
fn testing() {
    let mut app = spawn_app(1);
    let character_id = app.clients[0].character_id;

    let is_connected = |_: &mut World| -> bool { app.clients[0].client.is_connected() };
    app.run_until_condition_or_timeout(is_connected).unwrap();

    let enter_game_request = EnterGameRequest {
        token: "todo".to_string(),
        character_id,
    };
    app.client_send_message(0, DefaultChannel::ReliableOrdered, enter_game_request);

    let res = app.client_receive_message::<EnterGameResponse>(0, DefaultChannel::ReliableOrdered);
    assert_eq!(res.character_data.id, character_id);
}
