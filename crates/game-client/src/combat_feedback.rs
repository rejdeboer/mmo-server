use bevy::prelude::*;

const FLOAT_DURATION: f32 = 1.2;
const FLOAT_DISTANCE: f32 = 60.0;
const FLASH_DURATION: f32 = 0.15;
const FLASH_COLOR: LinearRgba = LinearRgba::new(4.0, 0.2, 0.2, 1.0);
const TEXT_OFFSET_Y: f32 = -40.0;

/// Internal message for combat hit feedback, emitted by network event handling.
#[derive(Message)]
pub struct CombatHitMessage {
    pub target_entity: Entity,
    pub amount: i32,
}

// ---------------------------------------------------------------------------
// Floating combat text
// ---------------------------------------------------------------------------

#[derive(Component)]
pub(crate) struct FloatingCombatText {
    target_entity: Entity,
    timer: Timer,
}

// ---------------------------------------------------------------------------
// Hit flash
// ---------------------------------------------------------------------------

#[derive(Component)]
pub(crate) struct HitFlash {
    timer: Timer,
    original_emissive: LinearRgba,
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Reads combat hit messages and spawns floating text + applies hit flash.
pub(crate) fn handle_combat_hits(
    mut commands: Commands,
    mut reader: MessageReader<CombatHitMessage>,
    q_targets: Query<&MeshMaterial3d<StandardMaterial>, Without<HitFlash>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for msg in reader.read() {
        // Spawn floating combat text
        commands.spawn((
            FloatingCombatText {
                target_entity: msg.target_entity,
                timer: Timer::from_seconds(FLOAT_DURATION, TimerMode::Once),
            },
            Text::new(format!("{}", msg.amount)),
            TextColor(Color::srgba(1.0, 0.9, 0.1, 1.0)),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
        ));

        // Apply hit flash
        if let Ok(material_handle) = q_targets.get(msg.target_entity)
            && let Some(material) = materials.get_mut(&material_handle.0)
        {
            let original_emissive = material.emissive;
            material.emissive = FLASH_COLOR;

            commands.entity(msg.target_entity).insert(HitFlash {
                timer: Timer::from_seconds(FLASH_DURATION, TimerMode::Once),
                original_emissive,
            });
        }
    }
}

/// Animates floating combat text: floats upward, fades out, then despawns.
pub(crate) fn update_floating_combat_text(
    mut commands: Commands,
    time: Res<Time>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    targets: Query<&GlobalTransform, Without<Camera>>,
    mut texts: Query<(Entity, &mut FloatingCombatText, &mut Node, &mut TextColor)>,
) {
    let Ok((camera, camera_global)) = cameras.single() else {
        return;
    };

    for (entity, mut fct, mut node, mut text_color) in texts.iter_mut() {
        fct.timer.tick(time.delta());

        if fct.timer.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = fct.timer.fraction();

        // Fade out over the last 40% of lifetime
        let alpha = if progress > 0.6 {
            1.0 - (progress - 0.6) / 0.4
        } else {
            1.0
        };
        text_color.0 = Color::srgba(1.0, 0.9, 0.1, alpha);

        // Position: project target world position to screen, float upward
        let Ok(target_transform) = targets.get(fct.target_entity) else {
            commands.entity(entity).despawn();
            continue;
        };

        // Offset above the entity's head (capsule height ~4 units)
        let world_pos = target_transform.translation() + Vec3::Y * 4.5;

        let Ok(viewport_pos) = camera.world_to_viewport(camera_global, world_pos) else {
            node.display = Display::None;
            continue;
        };

        node.display = Display::Flex;
        let float_offset = progress * FLOAT_DISTANCE;
        node.left = Val::Px(viewport_pos.x);
        node.top = Val::Px(viewport_pos.y + TEXT_OFFSET_Y - float_offset);
    }
}

/// Ticks hit flash timers and restores original material emissive when done.
pub(crate) fn update_hit_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut q_flashing: Query<(Entity, &mut HitFlash, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut flash, material_handle) in q_flashing.iter_mut() {
        flash.timer.tick(time.delta());

        if flash.timer.is_finished() {
            if let Some(material) = materials.get_mut(&material_handle.0) {
                material.emissive = flash.original_emissive;
            }
            commands.entity(entity).remove::<HitFlash>();
        }
    }
}
