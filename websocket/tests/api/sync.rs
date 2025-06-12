use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite;
use yrs::{
    updates::{decoder::Decode, encoder::Encode},
    Doc, GetString, ReadTxn, StateVector, Text, Transact, Update,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn other_client_receives_sync() {
    let app = spawn_app().await;
    let server_sv = StateVector::default();
    let mut client_a = app.create_owner_client().await;
    let mut client_b = app.create_owner_client().await;

    let doc_a = Doc::new();
    let text_a = doc_a.get_or_insert_text("test");

    {
        let mut txn = doc_a.transact_mut();
        text_a.push(&mut txn, "test");
    }

    let diff = doc_a.transact().encode_diff_v1(&server_sv);
    let mut update = diff.clone();
    update.push(websocket::websocket::MESSAGE_UPDATE);

    client_a
        .send(tungstenite::Message::Binary(update))
        .await
        .unwrap();

    match client_b.next().await.unwrap().unwrap() {
        tungstenite::Message::Binary(mut received) => match received.pop() {
            Some(websocket::websocket::MESSAGE_UPDATE) => assert_eq!(diff, received),
            other => panic!("expected an update message but got {other:?}"),
        },
        other => panic!("expected a binary message but got {other:?}"),
    };
}

#[tokio::test]
async fn get_diff_after_update() {
    let app = spawn_app().await;
    let server_sv = StateVector::default();

    let doc_a = Doc::new();
    let text_a = doc_a.get_or_insert_text("test");

    {
        let mut txn = doc_a.transact_mut();
        text_a.push(&mut txn, "test");
    }

    let mut update = doc_a.transact().encode_diff_v1(&server_sv);
    update.push(websocket::websocket::MESSAGE_UPDATE);

    let mut client_a = app.create_owner_client().await;
    client_a
        .send(tungstenite::Message::Binary(update.clone()))
        .await
        .unwrap();

    let doc_b = Doc::new();
    let mut sv_b = doc_b.transact().state_vector().encode_v1();
    sv_b.push(websocket::websocket::MESSAGE_SYNC_STEP_1);

    let mut client_b = app.create_owner_client().await;

    client_b
        .send(tungstenite::Message::Binary(sv_b))
        .await
        .unwrap();

    let mut received = match client_b.next().await.unwrap().unwrap() {
        tungstenite::Message::Binary(payload) => payload,
        other => panic!("expected a binary message but got {other:?}"),
    };

    let message_type = received.pop().unwrap();
    assert_eq!(message_type, websocket::websocket::MESSAGE_SYNC_STEP_2);

    doc_b
        .transact_mut()
        .apply_update(Update::decode_v1(&received).unwrap());
    let text_b = doc_b.get_or_insert_text("test");
    assert_eq!(
        text_a.get_string(&doc_a.transact()),
        text_b.get_string(&doc_b.transact())
    );
}
