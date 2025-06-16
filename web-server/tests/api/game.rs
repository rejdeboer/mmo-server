use web_server::routes::{CharacterCreate, CharacterRow, GameEntryRequest};

use crate::helpers::{spawn_app, TestApp};

#[tokio::test]
async fn game_entry_success() {
    let mut app = spawn_app().await;
    app.login().await;

    let character = app
        .create_character(CharacterCreate {
            name: "rejdeboer".to_string(),
        })
        .await
        .json::<CharacterRow>()
        .await
        .unwrap();

    let response = request_game_entry(
        &app,
        GameEntryRequest {
            character_id: character.id,
        },
    )
    .await;
    assert_eq!(response.status().as_u16(), 200);
}

async fn request_game_entry(app: &TestApp, body: GameEntryRequest) -> reqwest::Response {
    app.api_client
        .post(&format!("{}/game/request-entry", app.address))
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.")
}
