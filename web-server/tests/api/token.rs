use web_server::routes::LoginBody;

use crate::helpers::{spawn_app, TestAccount, TestApp};

#[tokio::test]
async fn login_success() {
    let app = spawn_app().await;
    let test_account = app.test_account().await;
    let response = login(&app, test_account).await;
    assert_eq!(response.status().as_u16(), 200);
}

async fn login(app: &TestApp, account: TestAccount) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("{}/token", app.address))
        .json(&LoginBody {
            email: account.email,
            password: account.password,
        })
        .send()
        .await
        .expect("Failed to execute request.")
}
