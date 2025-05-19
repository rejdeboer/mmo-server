use std::time::Instant;

use bevy::{prelude::*, tasks::Task};
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer, ServerEvent};
use bevy_tokio_tasks::TokioTasksRuntime;
use sqlx::{Pool, Postgres};

use crate::{application::DatabasePool, configuration::Settings};

#[derive(Debug, Component)]
pub struct ClientIdComponent(pub ClientId);

#[derive(Component)]
pub struct PendingConnection {
    client_id: ClientId,
    initiated_at: Instant,
}

#[derive(Component)]
pub struct HandshakeValidationTask(Task<Result<CharacterData, sqlx::Error>>);

#[derive(bincode::Decode, Debug)]
struct ClientHandshake {
    token: String,
    character_id: i32,
}

#[derive(Event, Debug)]
pub struct ProcessClientHandshake {
    client_id: ClientId,
    token: String,
    character_id: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct CharacterData {
    id: i32,
    name: String,
    position_x: f64,
    position_y: f64,
    position_z: f64,
    level: i32,
    experience: i64,
}

pub fn handle_connection_events(
    mut events: EventReader<ServerEvent>,
    mut commands: Commands,
    pending_connections_query: Query<(Entity, &PendingConnection)>,
    players: Query<(Entity, &ClientIdComponent, &Transform)>,
) {
    for event in events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                bevy::log::info!("player {} connected", client_id);
                commands.spawn(PendingConnection {
                    client_id: *client_id,
                    initiated_at: Instant::now(),
                });
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                bevy::log::info!("player {} disconnected: {}", client_id, reason);
                let client_id = *client_id;

                for (entity, pending_conn) in pending_connections_query.iter() {
                    if pending_conn.client_id == client_id {
                        commands.entity(entity).despawn();
                        return;
                    }
                }

                // TODO: Save character data
                for (entity, player_client_id, _transform) in players.iter() {
                    if player_client_id.0 == client_id {
                        commands.entity(entity).despawn();
                        return;
                    }
                }
            }
        }
    }
}

pub fn receive_initial_handshake_messages(
    mut server: ResMut<RenetServer>,
    pending_connections_query: Query<(Entity, &PendingConnection)>,
    mut event_writer: EventWriter<ProcessClientHandshake>,
    mut commands: Commands,
) {
    for (entity, pending_conn) in pending_connections_query.iter() {
        let client_id = pending_conn.client_id;
        while let Some(message_bytes) =
            server.receive_message(client_id, DefaultChannel::ReliableOrdered)
        {
            match bincode::decode_from_slice::<ClientHandshake, _>(
                &message_bytes,
                bincode::config::standard(),
            ) {
                Ok((decoded, _len)) => {
                    bevy::log::info!(
                        "received handshake from client {}: character_id {}, token {}",
                        client_id,
                        decoded.character_id,
                        decoded.token
                    );
                    event_writer.write(ProcessClientHandshake {
                        client_id,
                        character_id: decoded.character_id,
                        token: decoded.token,
                    });
                    commands.entity(entity).despawn();
                }
                Err(e) => {
                    bevy::log::error!(
                        "failed to deserialize handshake from client {}: {}; disconnecting",
                        client_id,
                        e
                    );
                    server.disconnect(client_id);
                    commands.entity(entity).despawn();
                }
            }
            break;
        }
    }
}

pub fn process_handshake_messages(
    mut server: ResMut<RenetServer>,
    mut handshake_reader: EventReader<ProcessClientHandshake>,
    runtime: Res<TokioTasksRuntime>,
    pool: Res<DatabasePool>,
    settings: Res<Settings>,
) {
    let is_secure = settings.server.is_secure;
    for event in handshake_reader.read() {
        let db_pool = pool.0.clone();
        let character_id = event.character_id;
        let client_id = event.client_id;
        runtime.spawn_background_task(async move |mut ctx| {
            if !is_secure {
                let character = load_character_data(db_pool, character_id).await.unwrap();
                ctx.run_on_main_thread(move |ctx| {
                    ctx.world.spawn((
                        ClientIdComponent(client_id),
                        Transform::from_xyz(
                            character.position_x as f32,
                            character.position_y as f32,
                            character.position_z as f32,
                        ),
                    ));
                })
                .await;
            }
            // TODO: Secure handshake validation using Redis
            unimplemented!()
        });
    }
}

async fn load_character_data(
    pool: Pool<Postgres>,
    character_id: i32,
) -> Result<CharacterData, sqlx::Error> {
    sqlx::query_as!(
        CharacterData,
        r#"
        SELECT id, name, level, experience,
            position_x, position_y, position_z
        FROM characters
        WHERE id = $1 
        "#,
        character_id,
    )
    .fetch_one(&pool)
    .await
}
