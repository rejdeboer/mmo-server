//! # Context Menu UI
//!
//! Reusable right-click context menu primitives. Provides the visual structure,
//! marker components, and hover effects. The caller is responsible for
//! registering domain-specific click observers on the spawned buttons.

use bevy::picking::events::{Out, Over, Pointer};
use bevy::prelude::*;

/// Marker for the context menu root node.
#[derive(Component)]
pub struct ContextMenu;

/// Marker for context menu buttons.
#[derive(Component)]
pub struct ContextMenuButton;

/// Despawns any existing context menu.
pub fn despawn_context_menu(commands: &mut Commands, existing: &Query<Entity, With<ContextMenu>>) {
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }
}

/// Spawns a context menu at the given screen position.
/// Returns the root entity so the caller can add children/buttons via
/// [`spawn_context_menu_button`].
pub fn spawn_context_menu(commands: &mut Commands, position: Vec2) -> Entity {
    commands
        .spawn((
            ContextMenu,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(position.x),
                top: Val::Px(position.y),
                flex_direction: FlexDirection::Column,
                min_width: Val::Px(150.0),
                padding: UiRect::all(Val::Px(4.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 0.95)),
            ZIndex(100),
            Pickable::IGNORE,
        ))
        .id()
}

/// Spawns a button inside a context menu. Returns the button entity so the
/// caller can attach their own click observer.
pub fn spawn_context_menu_button(
    commands: &mut Commands,
    menu_entity: Entity,
    label: &str,
) -> Entity {
    let button = commands
        .spawn((
            ContextMenuButton,
            Button,
            Node {
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                border_radius: BorderRadius::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ChildOf(menu_entity),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                Pickable::IGNORE,
            ));
        })
        .observe(on_button_hover_start)
        .observe(on_button_hover_end)
        .id();

    button
}

fn on_button_hover_start(event: On<Pointer<Over>>, mut bg_query: Query<&mut BackgroundColor>) {
    if let Ok(mut bg) = bg_query.get_mut(event.event_target()) {
        *bg = BackgroundColor(Color::srgba(0.3, 0.3, 0.5, 0.5));
    }
}

fn on_button_hover_end(event: On<Pointer<Out>>, mut bg_query: Query<&mut BackgroundColor>) {
    if let Ok(mut bg) = bg_query.get_mut(event.event_target()) {
        *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
    }
}
