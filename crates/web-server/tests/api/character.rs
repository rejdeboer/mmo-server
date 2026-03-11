use crate::helpers::spawn_app;
use web_client::WebClientError;
use web_types::CharacterCreate;

#[tokio::test]
async fn create_character_success() -> Result<(), WebClientError> {
    let mut app = spawn_app().await;
    app.login_account().await?;
    let character = app
        .client
        .create_character(&CharacterCreate {
            name: "rejdeboer".to_string(),
        })
        .await?;
    assert_eq!(character.name, "rejdeboer");
    Ok(())
}

#[tokio::test]
async fn create_character_with_character_token_success() -> Result<(), WebClientError> {
    let mut app = spawn_app().await;
    app.login_character().await?;
    let character = app
        .client
        .create_character(&CharacterCreate {
            name: "rejdeboer".to_string(),
        })
        .await?;
    assert_eq!(character.name, "rejdeboer");
    Ok(())
}

#[tokio::test]
async fn create_character_without_token_failure() -> Result<(), WebClientError> {
    let app = spawn_app().await;
    let result = app
        .client
        .create_character(&CharacterCreate {
            name: "rejdeboer".to_string(),
        })
        .await;
    assert!(matches!(result, Err(WebClientError::NotAuthenticated)));
    Ok(())
}
