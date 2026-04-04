use crate::net::{ConnectToken, ConnectionEvent, GameClient};
use protocol::client::MoveAction;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::Duration;
use tokio::time::Instant;
use tracing::instrument;

const TICK_DURATION: Duration = Duration::from_millis(1000 / 20);

/// How many ticks a bot holds the same heading before picking a new one.
const MIN_DIRECTION_TICKS: u32 = 20;
const MAX_DIRECTION_TICKS: u32 = 100;

/// Generates randomized movement inputs that change direction periodically.
struct BotMovement {
    rng: ChaCha8Rng,
    forward: f32,
    sideways: f32,
    yaw: f32,
    ticks_remaining: u32,
}

impl BotMovement {
    fn new(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            forward: 0.0,
            sideways: 0.0,
            yaw: 0.0,
            ticks_remaining: 0,
        }
    }

    /// Returns the next `MoveAction`, picking a new random direction if the
    /// current one has been held long enough.
    fn next_action(&mut self, tick: u32) -> MoveAction {
        if self.ticks_remaining == 0 {
            self.ticks_remaining = self
                .rng
                .random_range(MIN_DIRECTION_TICKS..MAX_DIRECTION_TICKS);
            self.forward = self.rng.random_range(-1.0..1.0_f32);
            self.sideways = self.rng.random_range(-1.0..1.0_f32);
            self.yaw = self.rng.random_range(0.0..std::f32::consts::TAU);
        } else {
            self.ticks_remaining -= 1;
        }

        MoveAction::from_f32(self.yaw, self.forward, self.sideways, tick)
    }
}

pub struct SimulatedClient {
    client: GameClient,
    client_id: u64,
    movement: BotMovement,
    tick: u32,
}

impl SimulatedClient {
    pub fn new(client_id: u64, seed: u64) -> Self {
        Self {
            client: GameClient::default(),
            client_id,
            movement: BotMovement::new(seed),
            tick: 0,
        }
    }

    #[instrument(skip_all, fields(client_id = self.client_id))]
    pub async fn run(mut self, connect_token: ConnectToken) -> anyhow::Result<()> {
        tracing::info!("starting bot");

        self.client.connect(connect_token);
        self.wait_for_enter_game().await?;
        self.run_game_loop().await;

        Ok(())
    }

    /// Poll the connection until we get an `EnterGameResponse` or disconnect.
    async fn wait_for_enter_game(&mut self) -> anyhow::Result<()> {
        let mut interval = tokio::time::interval(TICK_DURATION);
        let mut last_tick = Instant::now();

        loop {
            let now = interval.tick().await;
            let dt = now.duration_since(last_tick);
            last_tick = now;

            match self.client.poll_connection(dt) {
                Some(ConnectionEvent::EnterGameSuccess { player_name }) => {
                    tracing::info!(name = %player_name, "entered game");
                    return Ok(());
                }
                Some(ConnectionEvent::Disconnected) => {
                    anyhow::bail!("disconnected during connection phase");
                }
                None => {}
            }
        }
    }

    /// Send movement at 20Hz until the server disconnects us.
    async fn run_game_loop(&mut self) {
        let mut interval = tokio::time::interval(TICK_DURATION);
        let mut last_tick = Instant::now();

        loop {
            let now = interval.tick().await;
            let dt = now.duration_since(last_tick);
            last_tick = now;

            self.client.drain_messages(dt);

            if self.client.is_disconnected() {
                tracing::warn!("disconnected");
                break;
            }

            let action = self.movement.next_action(self.tick);
            self.client.send_movement(action);
            self.tick = self.tick.wrapping_add(1);
        }
    }
}
