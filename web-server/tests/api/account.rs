use web_server::routes::AccountCreate;

use crate::helpers::spawn_app;

#[tokio::test]
async fn create_account_success() {
    let app = spawn_app().await;
    let response = create_account(
        &app.address,
        AccountCreate {
            username: "rejdeboer".into(),
            email: "rick.deboer@live.nl".into(),
            password: "SuperSecret123!".into(),
        },
    )
    .await;
    assert_eq!(response.status().as_u16(), 200);
}

async fn create_account(address: &str, body: AccountCreate) -> reqwest::Response {
    reqwest::Client::new()
        .post(&format!("{}/account", address))
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.")
}
