pub mod interaction;
pub mod palette;
pub mod widgets;

use bevy::prelude::*;

use crate::application::AppState;

pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(interaction::plugin);

        app.add_systems(
            Update,
            widgets::update_unit_frames.run_if(in_state(AppState::InGame)),
        );
    }
}
