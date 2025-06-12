use futures::future::join_all;
use rand::seq::IteratorRandom;
use std::{collections::HashMap, ops::ControlFlow, sync::Arc};
use tokio::sync::mpsc::{Receiver, Sender};
use yrs::{
    updates::{decoder::Decode, encoder::Encode},
    StateVector, Update,
};

use sqlx::PgPool;
use tracing::{instrument, Instrument};
use uuid::Uuid;

use crate::server::ApplicationState;

use super::Message;

pub struct Syncer {
    clients: HashMap<Uuid, Sender<Vec<u8>>>,
    document_id: Uuid,
    state_vector: StateVector,
    rx: Receiver<Message>,
    state: Arc<ApplicationState>,
}

impl Syncer {
    pub fn new(
        state: Arc<ApplicationState>,
        document_id: Uuid,
        state_vector: Option<Vec<u8>>,
        rx: Receiver<Message>,
    ) -> Self {
        let state_vector = if let Some(sv) = state_vector {
            StateVector::decode_v1(&sv).expect("state vector decoded")
        } else {
            StateVector::default()
        };

        Self {
            clients: HashMap::new(),
            rx,
            state,
            document_id,
            state_vector,
        }
    }

    #[instrument(name="Syncer", parent=None, skip(self), fields(document_id=%self.document_id))]
    pub fn run(mut self) {
        tokio::spawn(
            async move {
                tracing::info!("starting syncer");
                while let Some(message) = self.rx.recv().await {
                    if self.process_message(message).await.is_break() {
                        tracing::info!("stopping syncer");
                        break;
                    };
                }
            }
            .instrument(tracing::Span::current()),
        );
    }

    async fn process_message(&mut self, message: Message) -> ControlFlow<(), ()> {
        match message {
            Message::Connect(id, tx) => {
                self.clients.insert(id, tx);
            }
            Message::Disconnect(id) => {
                self.clients.remove(&id);
                if self.clients.is_empty() {
                    let mut doc_handles = self
                        .state
                        .doc_handles
                        .lock()
                        .expect("receive doc_handles lock");
                    doc_handles.remove(&self.document_id);
                    return ControlFlow::Break(());
                };
            }
            Message::Update(id, mut update) => {
                self.forward_update(id, update.clone()).await;

                // Remove message type
                update.pop();

                self.store_update(update).await;
            }
            Message::GetDiff(id, mut state_vector) => {
                tracing::info!(%id, "received GetDiff message");
                let client_tx = self.clients.get(&id).expect("get client_tx").clone();

                // Pop the message type
                state_vector.pop();

                let mut diff =
                    compute_diff(state_vector, self.document_id, self.state.pool.clone()).await;

                diff.push(super::MESSAGE_SYNC_STEP_2);

                client_tx
                    .send(diff)
                    .await
                    .expect("SyncStep2 message sent to client");

                // Get diff from client
                let mut get_diff_msg = self.state_vector.encode_v1();
                get_diff_msg.push(super::MESSAGE_SYNC_STEP_1);
                client_tx
                    .send(get_diff_msg)
                    .await
                    .expect("SyncStep1 message sent to client");
            }
            Message::UpdateAwareness(id, update) => {
                self.forward_update(id, update).await;
            }
            Message::GetAwareness(id) => {
                // Pick random client to broadcast their awareness state
                let client_id = self
                    .clients
                    .keys()
                    .filter(|key| **key != id)
                    .choose(&mut rand::thread_rng());

                // If we got None, the sender is the only one editing the doc
                if let Some(client_id) = client_id {
                    let tx = self
                        .clients
                        .get(client_id)
                        .expect("client_handle retrieved for get awareness");
                    tx.send(vec![super::MESSAGE_GET_AWARENESS])
                        .await
                        .expect("get awareness message forwarded");
                }
            }
        };
        ControlFlow::Continue(())
    }

    async fn store_update(&mut self, update: Vec<u8>) {
        let pool = self.state.pool.clone();
        let document_id = self.document_id;

        let mut state_vector = Update::decode_v1(&update)
            .expect("update decoded")
            .state_vector();

        let mut txn = pool
            .begin()
            .await
            .expect("receive pool connection for update transaction");

        let current_clock = sqlx::query!(
            r#"
            SELECT COALESCE(MAX(clock), -1) as value 
            FROM document_updates
            WHERE document_id = $1;
            "#,
            document_id
        )
        .fetch_one(&mut *txn)
        .await
        .expect("retrieve current clock")
        .value
        .unwrap();

        state_vector.merge(self.state_vector.clone());

        let store_update = sqlx::query!(
            r#"
            INSERT INTO document_updates (document_id, clock, value)
            VALUES($1, $2, $3);
            "#,
            document_id,
            current_clock + 1,
            update
        )
        .execute(&mut *txn)
        .await;

        if store_update.is_err() {
            txn.rollback().await.expect("transaction rolled back");
            return;
        }

        sqlx::query!(
            r#"
            UPDATE documents
            SET state_vector = $2
            WHERE id = $1
            "#,
            self.document_id,
            state_vector.encode_v1()
        )
        .execute(&mut *txn)
        .await
        .expect("document updated");

        txn.commit().await.expect("transcation committed");

        self.state_vector.merge(state_vector);
    }

    async fn forward_update(&self, sender: Uuid, update: Vec<u8>) {
        join_all(
            self.clients
                .iter()
                .filter(|(client_id, _)| *client_id != &sender)
                .map(|(_, tx)| tx.send(update.clone()))
                .collect::<Vec<_>>(),
        )
        .await;
    }
}

async fn compute_diff(state_vector: Vec<u8>, document_id: Uuid, pool: PgPool) -> Vec<u8> {
    let encoded_updates = get_document_updates(document_id, pool).await;

    if encoded_updates.is_empty() {
        return vec![];
    }

    let updates = encoded_updates
        .into_iter()
        .map(|update| Update::decode_v1(&update).expect("update decoded"))
        .collect::<Vec<Update>>();

    let update = Update::merge_updates(updates).encode_v1();

    let diff =
        yrs::diff_updates_v1(update.as_slice(), state_vector.as_slice()).expect("computed diff");

    diff
}

async fn get_document_updates(document_id: Uuid, pool: PgPool) -> Vec<Vec<u8>> {
    sqlx::query!(
        r#"
        SELECT value
        FROM document_updates
        WHERE document_id = $1
        ORDER BY clock;
        "#,
        document_id
    )
    .fetch_all(&pool)
    .await
    .expect("retrieve document updates")
    .into_iter()
    .map(|update| update.value)
    .collect::<_>()
}
