use web_server::routes::CharacterCreate;

use crate::helpers::{spawn_app, TestApp};

#[tokio::test]
async fn create_account_success() {
    let mut app = spawn_app().await;
    app.login().await;
    let response = create_character(
        &app,
        CharacterCreate {
            name: "rejdeboer".to_string(),
        },
    )
    .await;
    assert_eq!(response.status().as_u16(), 200);
}

async fn create_character(app: &TestApp, body: CharacterCreate) -> reqwest::Response {
    app.api_client
        .post(&format!("{}/character", app.address))
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.")
}
