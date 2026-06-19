use bevy::prelude::*;
use bevy_renet::RenetClient;
use bevy_renet::renet::DefaultChannel;
use game_core::constants::TICK_RATE_HZ;
use protocol::client::PlayerAction;

const JITTER_BUFFER_TICKS: f32 = 2.0;
const MAX_RATE_ADJUSTMENT: f64 = 0.02;
const DRIFT_SMOOTHING: f32 = 0.1;
const BASE_TICK_DURATION: f64 = 1.0 / TICK_RATE_HZ;
const PING_INTERVAL_SECS: f32 = 2.;

/// Resource that keeps the client's tick synchronized with the server's tick.
///
/// The client runs in "server tick space" -- its tick counter represents its
/// best estimate of what server tick it should be simulating. The client
/// intentionally leads the server by a small amount so that inputs arrive
/// at the server just in time for processing.
#[derive(Resource, Debug)]
pub struct TickSync {
    pub tick: u32,
    target_lead: f32,
    drift_ema: f32,
    half_rtt_ticks_ema: f32,
}

impl Default for TickSync {
    fn default() -> Self {
        Self {
            tick: 0,
            target_lead: JITTER_BUFFER_TICKS,
            drift_ema: 0.0,
            half_rtt_ticks_ema: 1.0,
        }
    }
}

impl TickSync {
    pub fn new(server_tick: u32) -> Self {
        let initial_lead = (1.0 + JITTER_BUFFER_TICKS).ceil() as u32;

        Self {
            tick: server_tick.wrapping_add(initial_lead),
            target_lead: 1. + JITTER_BUFFER_TICKS,
            drift_ema: 0.,
            half_rtt_ticks_ema: 1.,
        }
    }

    pub fn increment(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn observe_pong(&mut self, server_tick: u32, client_tick_echo: u32) {
        let rtt_ticks = self.tick.wrapping_sub(client_tick_echo) as f32;
        let half_rtt = (rtt_ticks / 2.0).max(0.5);
        self.half_rtt_ticks_ema =
            self.half_rtt_ticks_ema * (1.0 - DRIFT_SMOOTHING) + half_rtt * DRIFT_SMOOTHING;

        self.target_lead = self.half_rtt_ticks_ema + JITTER_BUFFER_TICKS;

        let actual_lead = self.tick.wrapping_sub(server_tick) as i32 as f32;
        let drift = actual_lead - self.target_lead;

        self.drift_ema = self.drift_ema * (1.0 - DRIFT_SMOOTHING) + drift * DRIFT_SMOOTHING;
    }

    pub fn rate_adjustment(&self) -> f64 {
        let adjustment =
            (self.drift_ema as f64 * 0.01).clamp(-MAX_RATE_ADJUSTMENT, MAX_RATE_ADJUSTMENT);
        1.0 + adjustment
    }
}

pub fn increment_tick(mut tick_sync: ResMut<TickSync>) {
    tick_sync.increment();
}

pub fn adjust_tick_rate(tick_sync: Res<TickSync>, mut time: ResMut<Time<Fixed>>) {
    let adjustment = tick_sync.rate_adjustment();
    if (adjustment - 1.0).abs() > f64::EPSILON {
        let adjusted_duration = BASE_TICK_DURATION * adjustment;
        time.set_timestep_seconds(adjusted_duration);
    }
}

pub fn send_ping(
    tick_sync: Res<TickSync>,
    mut client: ResMut<RenetClient>,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    *timer += time.delta_secs();
    if *timer < PING_INTERVAL_SECS {
        return;
    }
    *timer -= PING_INTERVAL_SECS;

    let ping = PlayerAction::Ping {
        client_tick: tick_sync.tick,
    };
    let encoded = bitcode::encode(&ping);
    client.send_message(DefaultChannel::ReliableOrdered, encoded);
    tracing::debug!(client_tick = ?tick_sync.tick, "PING");
}
