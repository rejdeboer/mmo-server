use crate::{
    application::DatabasePool,
    components::{
        CharacterIdComponent, ClientIdComponent, InterestedClients, LevelComponent, NameComponent,
        VisibleEntities, Vitals,
    },
    telemetry::Metrics,
};
use bevy::prelude::*;
use bevy_renet::{
    netcode::NetcodeServerTransport,
    renet::{ClientId, DefaultChannel, DisconnectReason, RenetServer, ServerEvent},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use flatbuffers::{FlatBufferBuilder, UnionWIPOffset, WIPOffset, root};
use schemas::game::{self as schema};
use schemas::protocol::TokenUserData;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CharacterRow {
    pub id: i32,
    pub name: String,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub rotation_yaw: f32,
    pub level: i32,
    pub hp: i32,
    pub guild_id: Option<i32>,
}

#[derive(Debug)]
pub enum EntityAttributes {
    Player {
        character_id: i32,
        #[allow(dead_code)]
        guild_id: Option<i32>,
    },
    Npc,
}

impl EntityAttributes {
    pub fn serialize(
        &self,
        builder: &mut FlatBufferBuilder,
    ) -> (WIPOffset<UnionWIPOffset>, schema::EntityAttributes) {
        match self {
            EntityAttributes::Player {
                character_id,
                guild_id: _,
            } => {
                let fb_attr = schema::PlayerAttributes::create(
                    builder,
                    &schema::PlayerAttributesArgs {
                        character_id: *character_id,
                        guild_name: None,
                    },
                )
                .as_union_value();
                (fb_attr, schema::EntityAttributes::PlayerAttributes)
            }
            EntityAttributes::Npc => {
                let fb_attr = schema::NpcAttributes::create(builder, &schema::NpcAttributesArgs {})
                    .as_union_value();
                (fb_attr, schema::EntityAttributes::NpcAttributes)
            }
        }
    }
}

pub fn serialize_entity<'a>(
    builder: &mut FlatBufferBuilder<'a>,
    entity: Entity,
    attributes: &EntityAttributes,
    name: &str,
    transform: &Transform,
    vitals: &Vitals,
    level: i32,
) -> WIPOffset<schema::Entity<'a>> {
    let (fb_attr, attr_type) = attributes.serialize(builder);

    let pos = transform.translation;
    let fb_transform = schema::Transform::new(
        &schema::Vec3::new(pos.x, pos.y, pos.z),
        transform.rotation.y,
    );
    // TODO: Correctly instantiate vitals max hp according to entity stats
    let fb_vitals = schema::Vitals::new(vitals.hp, vitals.max_hp);
    let fb_name = builder.create_string(name);

    schema::Entity::create(
        builder,
        &schema::EntityArgs {
            id: entity.to_bits(),
            attributes_type: attr_type,
            attributes: Some(fb_attr),
            name: Some(fb_name),
            vitals: Some(&fb_vitals),
            transform: Some(&fb_transform),
            level,
        },
    )
}

#[allow(clippy::too_many_arguments)]
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
    metrics: Res<Metrics>,
) {
    for event in events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                metrics.connected_players.inc();
                process_client_connected(*client_id, &transport, &mut server, &pool, &runtime)
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                metrics.connected_players.dec();
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

    match root::<TokenUserData>(&user_data) {
        Ok(data) => {
            let character_id = data.character_id();
            let db_pool = pool.0.clone();
            runtime.spawn_background_task(async move |mut ctx| {
                let character = load_character_data(db_pool, character_id)
                    .await
                    .expect("player character data retrieved");
                ctx.run_on_main_thread(move |ctx| {
                    let mut transform = Transform::from_xyz(
                        character.position_x,
                        character.position_y,
                        character.position_z,
                    );
                    transform.rotate_y(character.rotation_yaw);
                    let vitals = Vitals {
                        hp: character.hp,
                        max_hp: character.hp,
                    };

                    let entity = ctx
                        .world
                        .spawn((
                            NameComponent(Arc::from(character.name.clone())),
                            ClientIdComponent(client_id),
                            CharacterIdComponent(character.id),
                            VisibleEntities::default(),
                            InterestedClients::default(),
                            transform,
                            vitals.clone(),
                            LevelComponent(character.level),
                        ))
                        .id();

                    let attributes = EntityAttributes::Player {
                        character_id: character.id,
                        guild_id: character.guild_id,
                    };

                    let mut builder = FlatBufferBuilder::new();
                    let entity_offset = serialize_entity(
                        &mut builder,
                        entity,
                        &attributes,
                        &character.name,
                        &transform,
                        &vitals,
                        character.level,
                    );
                    let response_offset = schema::EnterGameResponse::create(
                        &mut builder,
                        &schema::EnterGameResponseArgs {
                            player_entity: Some(entity_offset),
                        },
                    );
                    builder.finish_minimal(response_offset);
                    let response = builder.finished_data().to_vec();

                    let mut server = ctx.world.get_resource_mut::<RenetServer>().unwrap();
                    info!("approving enter game request by client {}", client_id);
                    server.send_message(client_id, DefaultChannel::ReliableOrdered, response);
                })
                .await;
            });
        }
        Err(error) => {
            error!(
                ?error,
                "failed to deserialize user data from client {}; disconnecting", client_id,
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
        SELECT id, guild_id, name, level, hp,
            position_x, position_y, position_z,
            rotation_yaw
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
            let transform = *transform;
            commands.entity(entity).despawn();
            runtime.spawn_background_task(async move |_| {
                // TODO: This pos is probably incorrect
                let pos = transform.translation;
                let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
                if let Err(error) = sqlx::query!(
                    r#"
                    UPDATE CHARACTERS
                    SET position_x = $2, position_y = $3, position_z = $4,
                        rotation_yaw = $5
                    WHERE id = $1 
                    "#,
                    character_id,
                    pos.x,
                    pos.y,
                    pos.z,
                    yaw,
                )
                .execute(&db_pool)
                .await
                {
                    error!(?error, "failed to update character");
                };
            });
            return;
        }
    }
}
