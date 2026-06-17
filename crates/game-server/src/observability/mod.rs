mod metrics;

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use std::time::Duration;

pub struct ObservabilityPlugin;

impl Plugin for ObservabilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            metrics::update_server_metrics.run_if(on_timer(Duration::from_secs(5))),
        );
    }
}
