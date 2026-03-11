use crate::helpers::spawn_app;
use web_client::WebClientError;
use web_types::CharacterCreate;

#[tokio::test]
async fn game_entry_success() -> Result<(), WebClientError> {
    let mut app = spawn_app().await;
    app.login_account().await?;

    let character = app
        .client
        .create_character(&CharacterCreate {
            name: "rejdeboer".to_string(),
        })
        .await?;

    app.client.select_character(character.id).await
}
