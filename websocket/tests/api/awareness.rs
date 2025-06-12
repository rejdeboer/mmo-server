use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite;
use websocket::websocket;

use crate::helpers::spawn_app;

#[tokio::test]
async fn other_client_receives_query_awareness() {
    let app = spawn_app().await;
    let mut client_a = app.create_owner_client().await;
    let mut client_b = app.create_owner_client().await;

    let message = vec![websocket::MESSAGE_GET_AWARENESS];

    client_a
        .send(tungstenite::Message::Binary(message.clone()))
        .await
        .unwrap();

    match client_b.next().await.unwrap().unwrap() {
        tungstenite::Message::Binary(mut received) => {
            assert_eq!(received.pop(), Some(websocket::MESSAGE_GET_AWARENESS))
        }
        other => panic!("expected a binary message but got {other:?}"),
    };
}
