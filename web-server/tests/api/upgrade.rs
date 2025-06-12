use tokio_tungstenite::tungstenite;
use uuid::Uuid;

use crate::helpers::spawn_app;

#[tokio::test]
async fn unauthorized() {
    let app = spawn_app().await;
    let test_doc = app.test_document().await;
    let request = app.create_connection_request("unauthorized-token".to_string(), test_doc.0);

    let error = tokio_tungstenite::connect_async(request)
        .await
        .expect_err("401 received");

    match error {
        tungstenite::Error::Http(response) => assert_eq!(response.status(), 401),
        other => panic!("expected an http error message but got {other:?}"),
    }
}

#[tokio::test]
async fn doc_not_found() {
    let app = spawn_app().await;
    let test_doc = app.test_document().await;
    let request = app.create_connection_request(app.signed_jwt(test_doc.1), Uuid::new_v4());

    let error = tokio_tungstenite::connect_async(request)
        .await
        .expect_err("404 received");

    match error {
        tungstenite::Error::Http(response) => assert_eq!(response.status(), 404),
        other => panic!("expected an http error message but got {other:?}"),
    }
}

#[tokio::test]
async fn user_has_no_access() {
    let app = spawn_app().await;
    let test_doc = app.test_document().await;
    let request = app.create_connection_request(app.signed_jwt(Uuid::new_v4()), test_doc.0);

    let error = tokio_tungstenite::connect_async(request)
        .await
        .expect_err("404 received");

    match error {
        tungstenite::Error::Http(response) => assert_eq!(response.status(), 404),
        other => panic!("expected an http error message but got {other:?}"),
    }
}
