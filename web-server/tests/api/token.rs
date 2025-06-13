use web_server::routes::LoginBody;

use crate::helpers::{spawn_app, TestApp};

#[tokio::test]
async fn login_success() {
    let app = spawn_app().await;
    let response = login(&app).await;
    assert_eq!(response.status().as_u16(), 200);
}

async fn login(app: &TestApp) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("{}/token", app.address))
        .json(&LoginBody {
            email: app.account.email.clone(),
            password: app.account.password.clone(),
        })
        .send()
        .await
        .expect("Failed to execute request.")
}
