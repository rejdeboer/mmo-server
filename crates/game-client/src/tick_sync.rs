use bevy::prelude::*;
use bevy_renet::renet::DefaultChannel;
use bevy_renet::RenetClient;
use game_core::constants::TICK_RATE_HZ;
use protocol::client::PlayerAction;

/// Number of extra ticks the client should lead the server by (jitter buffer).
/// A value of 2 means the client aims to be ~2 ticks ahead of the server,
/// so inputs arrive with a small buffer against network jitter.
const JITTER_BUFFER_TICKS: f32 = 2.0;

/// Maximum rate adjustment factor. The client will speed up or slow down
/// its FixedUpdate timestep by at most this fraction (e.g., 0.02 = 2%).
const MAX_RATE_ADJUSTMENT: f64 = 0.02;

/// Smoothing factor for the exponential moving average of drift samples.
/// Lower values = smoother but slower to react. Higher = more responsive but jittery.
const DRIFT_SMOOTHING: f32 = 0.1;

const BASE_TICK_DURATION: f64 = 1.0 / TICK_RATE_HZ;
const PING_INTERVAL_SECS: f32 = 2.;

/// Resource that keeps the client's tick synchronized with the server's tick.
///
/// The client runs in "server tick space" -- its tick counter represents its
/// best estimate of what server tick it should be simulating. The client
/// intentionally leads the server by a small amount so that inputs arrive
/// at the server just in time for processing.
///
/// ## How it works
///
/// 1. **Initial sync**: When the client receives `EnterGameResponse`, it seeds
///    its tick from `server_tick + target_lead`. This gets the client into the
///    right ballpark immediately.
///
/// 2. **Ongoing correction**: Each `ServerMovementPayload` contains the server's
///    current tick. The client compares `client_tick - server_tick` against its
///    `target_lead`. If the client is drifting too far ahead or behind, it
///    adjusts the `Time<Fixed>` timestep slightly (±1-2%) to converge.
///
/// 3. **Rate adjustment** (not teleportation): Rather than jumping the tick
///    counter, the client adjusts its simulation *rate*. This avoids visual
///    hitches from skipping or repeating ticks.
#[derive(Resource, Debug)]
pub struct TickSync {
    /// The client's current tick in server-tick space.
    /// Incremented once per FixedUpdate.
    pub tick: u32,

    /// How many ticks ahead of the server the client aims to be.
    /// Computed as `(RTT_estimate / 2) / tick_duration + JITTER_BUFFER_TICKS`.
    target_lead: f32,

    /// Exponential moving average of the observed drift
    /// (`client_tick - latest_server_tick - target_lead`).
    /// Positive = client is too far ahead. Negative = client is too far behind.
    drift_ema: f32,

    /// Running estimate of the half-RTT in ticks, used to compute `target_lead`.
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

    /// Advance the tick counter. Called once per FixedUpdate.
    pub fn increment(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    /// Process an incoming `Pong` from the server and update the drift estimate.
    ///
    /// `server_tick` is the server's tick at the time it sent the pong.
    /// `client_tick_echo` is the client tick we originally sent in the ping --
    /// this gives us an RTT estimate: `current_tick - client_tick_echo`.
    pub fn observe_pong(&mut self, server_tick: u32, client_tick_echo: u32) {
        // RTT in ticks = how many ticks have passed since we sent the ping.
        let rtt_ticks = self.tick.wrapping_sub(client_tick_echo) as f32;
        let half_rtt = (rtt_ticks / 2.0).max(0.5);
        self.half_rtt_ticks_ema =
            self.half_rtt_ticks_ema * (1.0 - DRIFT_SMOOTHING) + half_rtt * DRIFT_SMOOTHING;

        // Update target lead based on latest half-RTT estimate.
        self.target_lead = self.half_rtt_ticks_ema + JITTER_BUFFER_TICKS;

        // Compute how far off we are from the target.
        // `actual_lead = client_tick - server_tick`
        // `drift = actual_lead - target_lead`
        // Positive drift = we're too far ahead, negative = too far behind.
        let actual_lead = self.tick.wrapping_sub(server_tick) as i32 as f32;
        let drift = actual_lead - self.target_lead;

        self.drift_ema = self.drift_ema * (1.0 - DRIFT_SMOOTHING) + drift * DRIFT_SMOOTHING;
    }

    /// Compute the rate adjustment factor to apply to `Time<Fixed>`.
    ///
    /// Returns a multiplier for the tick duration:
    /// - `> 1.0` means slow down (client is too far ahead)
    /// - `< 1.0` means speed up (client is too far behind)
    /// - `1.0` means no adjustment needed
    pub fn rate_adjustment(&self) -> f64 {
        // Proportional control: adjust rate proportional to drift.
        // If drift_ema > 0 (too far ahead), we want to slow down (multiply dt by > 1).
        // If drift_ema < 0 (too far behind), we want to speed up (multiply dt by < 1).
        //
        // Scale: 1 tick of drift → ~1% adjustment.
        let adjustment =
            (self.drift_ema as f64 * 0.01).clamp(-MAX_RATE_ADJUSTMENT, MAX_RATE_ADJUSTMENT);
        1.0 + adjustment
    }
}

pub fn increment_tick(mut tick_sync: ResMut<TickSync>) {
    tick_sync.increment();
}

/// System that adjusts the `Time<Fixed>` timestep based on drift.
/// Runs in `Update` so it takes effect on the next FixedUpdate cycle.
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
