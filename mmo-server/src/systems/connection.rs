use bevy::prelude::*;
use bevy_renet::{
    netcode::NetcodeServerTransport,
    renet::{ClientId, DefaultChannel, DisconnectReason, RenetServer, ServerEvent},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use flatbuffers::{FlatBufferBuilder, WIPOffset, root};
use sqlx::{Pool, Postgres};

use crate::{
    application::{DatabasePool, EntityIdCounter},
    components::{CharacterIdComponent, ClientIdComponent, EntityId},
};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CharacterRow {
    pub id: i32,
    pub name: String,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub level: i32,
    pub experience: i64,
}

impl CharacterRow {
    fn encode<'a>(
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

pub fn handle_connection_events(
    mut events: EventReader<ServerEvent>,
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    transport: Res<NetcodeServerTransport>,
    players: Query<(
        Entity,
        &ClientIdComponent,
        &CharacterIdComponent,
        &Transform,
    )>,
    runtime: Res<TokioTasksRuntime>,
    pool: Res<DatabasePool>,
) {
    for event in events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                process_client_connected(*client_id, &transport, &mut server, &pool, &runtime)
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                process_client_disconnected(
                    *client_id,
                    reason,
                    &mut commands,
                    players,
                    &pool,
                    &runtime,
                );
            }
        }
    }
}

fn process_client_connected(
    client_id: ClientId,
    transport: &NetcodeServerTransport,
    server: &mut RenetServer,
    pool: &DatabasePool,
    runtime: &TokioTasksRuntime,
) {
    bevy::log::info!("player {} connected", client_id);
    let user_data_option = transport.user_data(client_id);
    if user_data_option.is_none() {
        return server.disconnect(client_id);
    }
    let user_data = user_data_option.unwrap();

    match root::<schemas::mmo::NetcodeTokenUserData>(&user_data) {
        Ok(data) => {
            let character_id = data.character_id();
            let db_pool = pool.0.clone();
            let client_id = client_id;
            runtime.spawn_background_task(async move |mut ctx| {
                let character = load_character_data(db_pool, character_id).await.unwrap();
                ctx.run_on_main_thread(move |ctx| {
                    let entity_id = ctx
                        .world
                        .get_resource_mut::<EntityIdCounter>()
                        .unwrap()
                        .increment();
                    ctx.world.spawn((
                        ClientIdComponent(client_id),
                        EntityId(entity_id),
                        CharacterIdComponent(character.id),
                        Transform::from_xyz(
                            character.position_x,
                            character.position_y,
                            character.position_z,
                        ),
                    ));

                    let mut builder = FlatBufferBuilder::new();
                    let character_offset = character.encode(&mut builder);
                    let response_offset = schemas::mmo::EnterGameResponse::create(
                        &mut builder,
                        &schemas::mmo::EnterGameResponseArgs {
                            player_entity_id: entity_id,
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
            });
        }
        Err(error) => {
            bevy::log::error!(
                ?error,
                "failed to deserialize user data from client {}; disconnecting",
                client_id,
            );
            server.disconnect(client_id);
        }
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

fn process_client_disconnected(
    client_id: ClientId,
    reason: &DisconnectReason,
    commands: &mut Commands,
    players: Query<(
        Entity,
        &ClientIdComponent,
        &CharacterIdComponent,
        &Transform,
    )>,
    pool: &DatabasePool,
    runtime: &TokioTasksRuntime,
) {
    bevy::log::info!("player {} disconnected: {}", client_id, reason);

    for (entity, player_client_id, character_id, transform) in players.iter() {
        if player_client_id.0 == client_id {
            let db_pool = pool.0.clone();
            let character_id = character_id.0;
            let transform = transform.clone();
            commands.entity(entity).despawn();
            runtime.spawn_background_task(async move |_| {
                // TODO: This pos is probably incorrect
                let pos = transform.translation;
                if let Err(error) = sqlx::query!(
                    r#"
                    UPDATE CHARACTERS
                    SET position_x = $2, position_y = $3, position_z = $4
                    WHERE id = $1 
                    "#,
                    character_id,
                    pos.x,
                    pos.y,
                    pos.z,
                )
                .execute(&db_pool)
                .await
                {
                    bevy::log::error!(?error, "failed to update character");
                };
            });
            return;
        }
    }
}
