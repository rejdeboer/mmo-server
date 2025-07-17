use web_server::routes::CharacterCreate;

use crate::helpers::spawn_app;

#[tokio::test]
async fn create_character_success() {
    let mut app = spawn_app().await;
    app.login_account().await;
    let response = app
        .create_character(CharacterCreate {
            name: "rejdeboer".to_string(),
        })
        .await;
    assert_eq!(response.status().as_u16(), 200);
}
