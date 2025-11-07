use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use std::time::Duration;

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins);
        app.add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            Duration::from_secs_f64(crate::application::TICK_SECS),
        )));
        info!("running in debug mode");
    }
}
