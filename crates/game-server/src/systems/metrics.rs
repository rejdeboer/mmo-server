use crate::telemetry::{
    CONNECTED_PLAYERS_METRIC, SERVER_RTT_METRIC, SERVER_SIMULATION_TICK_METRIC,
};
use bevy::prelude::*;
use bevy_renet::RenetServer;

pub fn update_server_metrics(server: Res<RenetServer>) {
    metrics::gauge!(CONNECTED_PLAYERS_METRIC).set(server.connected_clients() as u32);
    for client_id in server.clients_id() {
        metrics::histogram!(SERVER_RTT_METRIC).record(server.rtt(client_id));
    }
}

pub fn increment_simulation_tick() {
    metrics::counter!(SERVER_SIMULATION_TICK_METRIC).increment(1);
}
