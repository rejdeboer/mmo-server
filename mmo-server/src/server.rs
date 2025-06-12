use std::time::Instant;

use bevy::{prelude::*, tasks::Task};
use bevy_renet::{
    netcode::NetcodeServerTransport,
    renet::{ClientId, DefaultChannel, RenetServer, ServerEvent},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use flatbuffers::{FlatBufferBuilder, WIPOffset, root};
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
pub struct EnterGameValidationTask(Task<Result<CharacterRow, sqlx::Error>>);

#[derive(Event, Debug)]
pub struct EnterGameEvent {
    client_id: ClientId,
    token: String,
    character_id: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CharacterRow {
    pub id: i32,
    pub name: String,
    pub position_x: f64,
    pub position_y: f64,
    pub position_z: f64,
    pub level: i32,
    pub experience: i64,
}

impl CharacterRow {
    pub fn serialize<'a>(
        self,
        builder: &mut FlatBufferBuilder<'a>,
    ) -> WIPOffset<schemas::mmo::Character<'a>> {
        let transform = schemas::mmo::Transform::new(
            &schemas::mmo::Vec3::new(self.position_x, self.position_y, self.position_z),
            &schemas::mmo::Vec3::new(0., 0., 0.),
        );
        let name = builder.create_string(&self.name);

        let entity = schemas::mmo::Entity::create(
            builder,
            &schemas::mmo::EntityArgs {
                name: Some(name),
                // TODO: Fill this in
                hp: 0,
                level: self.level,
                transform: Some(&transform),
            },
        );

        schemas::mmo::Character::create(
            builder,
            &schemas::mmo::CharacterArgs {
                entity: Some(entity),
            },
        )
    }
}

pub fn send_packets(
    mut server: ResMut<RenetServer>,
    mut transport: ResMut<NetcodeServerTransport>,
) {
    transport.send_packets(&mut server);
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

pub fn receive_enter_game_requests(
    mut server: ResMut<RenetServer>,
    pending_connections_query: Query<(Entity, &PendingConnection)>,
    mut event_writer: EventWriter<EnterGameEvent>,
    mut commands: Commands,
) {
    for (entity, pending_conn) in pending_connections_query.iter() {
        let client_id = pending_conn.client_id;
        while let Some(message_bytes) =
            server.receive_message(client_id, DefaultChannel::ReliableOrdered)
        {
            // TODO: Should we verify the file identifier?
            match root::<schemas::mmo::EnterGameRequest>(&message_bytes) {
                Ok(request) => {
                    let character_id = request.character_id();
                    let token = request.token().unwrap();
                    bevy::log::info!(
                        "received handshake from client {}: character_id {}, token {}",
                        client_id,
                        character_id,
                        token.to_string(),
                    );
                    event_writer.write(EnterGameEvent {
                        client_id,
                        character_id,
                        token: token.to_string(),
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

pub fn process_enter_game_requests(
    mut event_reader: EventReader<EnterGameEvent>,
    runtime: Res<TokioTasksRuntime>,
    pool: Res<DatabasePool>,
    settings: Res<Settings>,
) {
    let is_secure = settings.server.is_secure;
    for event in event_reader.read() {
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

                    let mut builder = FlatBufferBuilder::new();
                    let character_offset = character.serialize(&mut builder);
                    let response_offset = schemas::mmo::EnterGameResponse::create(
                        &mut builder,
                        &schemas::mmo::EnterGameResponseArgs {
                            character: Some(character_offset),
                        },
                    );
                    builder.finish_minimal(response_offset);
                    let response = builder.finished_data().to_vec();

                    let mut server = ctx.world.get_resource_mut::<RenetServer>().unwrap();
                    bevy::log::info!("approving enter game request by client {}", client_id);
                    server.send_message(client_id, DefaultChannel::ReliableOrdered, response);
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
) -> Result<CharacterRow, sqlx::Error> {
    sqlx::query_as!(
        CharacterRow,
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
