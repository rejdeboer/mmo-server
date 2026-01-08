use crate::{
    application::DatabasePool,
    collision::GameLayer,
    components::{
        CharacterIdComponent, ClientIdComponent, InterestedClients, LevelComponent,
        MovementSpeedComponent, NameComponent, VisibleEntities, Vitals,
    },
    database::load_character_data,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_renet::{
    netcode::NetcodeServerTransport,
    renet::{ClientId, DefaultChannel, DisconnectReason, RenetServer, ServerEvent},
};
use bevy_tokio_tasks::{TaskContext, TokioTasksRuntime};
use flatbuffers::{FlatBufferBuilder, UnionWIPOffset, WIPOffset, root};
use protocol::{
    models::{Actor, ActorAttributes},
    primitives::Transform as NetTransform,
    server::EnterGameResponse,
};
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tracing::{Instrument, Level, instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;

// TODO: This should probably be done in another module
const SPEED_PRECISION_MULTIPLIER: f32 = 100.;
const BASE_MOVEMENT_SPEED: f32 = 7.5;

#[derive(Bundle)]
/// Base components used by entities that interact with the world, like players, monsters, NPCs
pub struct ActorBundle {
    name: NameComponent,
    transform: Transform,
    vitals: Vitals,
    movement_speed: MovementSpeedComponent,
    level: LevelComponent,
    interested_clients: InterestedClients,
    body: RigidBody,
    collider: Collider,
    collision_layers: CollisionLayers,
    shape_caster: ShapeCaster,
    locked_axes: LockedAxes,
}

impl ActorBundle {
    pub fn new(name: &str, transform: Transform, vitals: Vitals, level: i32) -> Self {
        Self {
            name: NameComponent(Arc::from(name)),
            transform,
            vitals: vitals.clone(),
            movement_speed: MovementSpeedComponent(BASE_MOVEMENT_SPEED),
            level: LevelComponent(level),
            interested_clients: InterestedClients::default(),
            body: RigidBody::Dynamic,
            locked_axes: LockedAxes::ROTATION_LOCKED,
            collider: Collider::capsule(1., 2.),
            collision_layers: CollisionLayers::new(
                GameLayer::Player,
                [GameLayer::Default, GameLayer::Ground],
            ),
            shape_caster: ShapeCaster::new(
                Collider::capsule(0.9, 0.1),
                Vec3::ZERO,
                Quat::IDENTITY,
                Dir3::NEG_Y,
            )
            .with_query_filter(SpatialQueryFilter::from_mask(LayerMask::ALL)),
        }
    }
}

#[derive(Bundle)]
pub struct CharacterBundle {
    base: ActorBundle,
    id: CharacterIdComponent,
    client_id: ClientIdComponent,
    visible_entities: VisibleEntities,
}

impl CharacterBundle {
    pub fn new(base: ActorBundle, id: i32, client_id: u64) -> Self {
        Self {
            base,
            id: CharacterIdComponent(id),
            client_id: ClientIdComponent(client_id),
            visible_entities: VisibleEntities::default(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_connection_events(
    mut events: MessageReader<ServerEvent>,
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
    let user_data_option = transport.user_data(client_id);
    if user_data_option.is_none() {
        return server.disconnect(client_id);
    }
    let user_data = user_data_option.unwrap();

    match root::<TokenUserData>(&user_data) {
        Ok(data) => {
            let character_id = data.character_id();
            let db_pool = pool.0.clone();

            let span =
                tracing::span!(Level::INFO, "process_client_connected", client_id = %client_id);
            if let Some(traceparent) = data.traceparent() {
                let mut headers = HashMap::new();
                headers.insert("traceparent".to_string(), traceparent.to_string());
                let parent_ctx =
                    opentelemetry::global::get_text_map_propagator(|p| p.extract(&headers));
                if let Err(err) = span.set_parent(parent_ctx) {
                    tracing::error!(?err, "failed to set otel span parent");
                };
            }

            runtime.spawn_background_task(async move |ctx| {
                handle_enter_game_task(db_pool, client_id, character_id, ctx)
                    .instrument(span)
                    .await
            });
        }
        Err(error) => {
            tracing::error!(
                ?error,
                ?client_id,
                "failed to deserialize user data, disconnecting",
            );
            server.disconnect(client_id);
        }
    }
}

#[instrument(skip_all, fields(character_id = %character_id))]
async fn handle_enter_game_task(
    pool: PgPool,
    client_id: ClientId,
    character_id: i32,
    mut ctx: TaskContext,
) {
    tracing::info!("entering game");
    let character = load_character_data(pool, character_id)
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
            max_hp: character.max_hp,
        };

        let entity = ctx
            .world
            .spawn(CharacterBundle::new(
                ActorBundle::new(&character.name, transform, vitals.clone(), character.level),
                character.id,
                client_id,
            ))
            .id();

        let attributes = ActorAttributes::Player {
            character_id: character.id,
            // TODO: Handle guild fetching
            guild_name: None,
        };

        let player_actor = Actor {
            id: entity.to_bits(),
            attributes,
            name: character.name,
            transform: NetTransform::from_glam(transform.translation, transform.rotation),
            vitals: vitals.into(),
            level: character.level as u8,
            movement_speed: BASE_MOVEMENT_SPEED.into(),
        };

        let response = EnterGameResponse { player_actor };

        let mut server = ctx.world.get_resource_mut::<RenetServer>().unwrap();
        tracing::info!("approving enter game request for client {}", client_id);
        server.send_message(
            client_id,
            DefaultChannel::ReliableOrdered,
            bitcode::encode(&response),
        );
    })
    .await;
    tracing::info!("successfully sent EnterGameResponse");
}

#[instrument(skip_all, fields(client_id = client_id))]
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
    tracing::info!(?reason, "client disconnected");

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
                    tracing::error!(?error, "failed to update character");
                };
            });
            return;
        }
    }
}
