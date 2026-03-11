use crate::telemetry::Metrics;
use bevy::prelude::*;
use bevy_renet::RenetServer;

pub fn update_server_metrics(server: Res<RenetServer>, metrics: ResMut<Metrics>) {
    metrics
        .connected_players
        .set(server.connected_clients() as i64);

    for client_id in server.clients_id() {
        metrics.server_rtt.observe(server.rtt(client_id));
    }
}
