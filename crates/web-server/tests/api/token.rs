use web_client::WebClientError;
use web_types::LoginBody;

use crate::helpers::spawn_app;

#[tokio::test]
async fn login_success() -> Result<(), WebClientError> {
    let mut app = spawn_app().await;
    app.client
        .login(&LoginBody {
            email: app.account.email.clone(),
            password: app.account.password.clone(),
        })
        .await
}

#[tokio::test]
async fn login_wrong_password() {
    let mut app = spawn_app().await;
    let result = app
        .client
        .login(&LoginBody {
            email: app.account.email.clone(),
            password: "wrong".to_string(),
        })
        .await;

    let Err(WebClientError::ApiError { status, message: _ }) = result else {
        panic!("Expected API error");
    };
    assert_eq!(status.as_u16(), 401);
}

#[tokio::test]
async fn login_wrong_email() {
    let mut app = spawn_app().await;
    let result = app
        .client
        .login(&LoginBody {
            email: "wrong@wrong.wrong".to_string(),
            password: app.account.password.clone(),
        })
        .await;

    let Err(WebClientError::ApiError { status, message: _ }) = result else {
        panic!("Expected API error");
    };
    assert_eq!(status.as_u16(), 401);
}
