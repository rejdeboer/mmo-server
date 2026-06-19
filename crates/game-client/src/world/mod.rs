mod scene;
mod actors;
pub mod camera;
pub mod selection;
mod target_frame;
mod player_frame;

use bevy::prelude::*;

use crate::application::AppState;
use crate::networking::NetworkingSet;

/// Shared capsule mesh handle used for debug placeholder rendering of all actors.
#[derive(Resource)]
pub struct DebugActorMesh(pub Handle<Mesh>);

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(selection::SelectedTarget::default());
        app.insert_resource(selection::ClickTracker::default());
        app.insert_resource(selection::CursorOverUi::default());

        app.add_systems(Startup, scene::setup_world);

        app.add_systems(
            Update,
            (
                selection::update_cursor_over_ui,
                camera::camera_input,
                camera::manage_cursor_grab,
                camera::update_camera_transform,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        );

        app.add_systems(
            Update,
            (
                actors::handle_actor_spawn_messages,
                actors::handle_actor_despawn_messages,
            )
                .after(NetworkingSet::Receive)
                .run_if(in_state(AppState::InGame)),
        );

        app.add_systems(
            Update,
            (
                selection::handle_target_selection,
                target_frame::manage_target_unit_frame,
                target_frame::sync_target_unit_frame,
                target_frame::handle_target_context_menu,
            player_frame::spawn_player_unit_frame,
            player_frame::handle_player_context_menu,
        )
                .run_if(in_state(AppState::InGame)),
        );

        app.add_systems(
            Update,
            selection::clear_despawned_target.run_if(in_state(AppState::InGame)),
        );
    }
}
