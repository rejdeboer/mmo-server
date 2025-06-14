use web_server::routes::LoginBody;

use crate::helpers::{spawn_app, TestApp};

#[tokio::test]
async fn login_success() {
    let app = spawn_app().await;
    let response = login(
        &app,
        app.account.email.clone(),
        app.account.password.clone(),
    )
    .await;
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn login_wrong_password() {
    let app = spawn_app().await;
    let response = login(&app, app.account.email.clone(), "wrong".to_string()).await;
    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn login_wrong_email() {
    let app = spawn_app().await;
    let response = login(
        &app,
        "wrong@wrong.wrong".to_string(),
        app.account.password.clone(),
    )
    .await;
    assert_eq!(response.status().as_u16(), 401);
}

async fn login(app: &TestApp, email: String, password: String) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("{}/token", app.address))
        .json(&LoginBody { email, password })
        .send()
        .await
        .expect("Failed to execute request.")
}
