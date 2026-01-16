use mmo_client::{
    ConnectToken, ConnectionEvent, GameClient,
    protocol::{client::MoveAction, models::Actor},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::Duration;
use tokio::time::Instant;
use tracing::instrument;

const TICK_DURATION: Duration = Duration::from_millis(1000 / 20);

pub enum SimulatedClientState {
    Disconnected,
    Connected(Actor),
}

pub struct SimulatedClient {
    client: GameClient,
    client_id: u64,
    rng: ChaCha8Rng,
    state: SimulatedClientState,
}

impl SimulatedClient {
    pub fn new(client_id: u64, seed: u64) -> Self {
        let client = GameClient::default();
        let rng = ChaCha8Rng::seed_from_u64(seed);

        Self {
            client,
            client_id,
            rng,
            state: SimulatedClientState::Disconnected,
        }
    }

    #[instrument(skip_all, fields(client_id = self.client_id))]
    pub async fn run(mut self, connect_token: ConnectToken) -> anyhow::Result<()> {
        tracing::info!("starting bot");

        self.client.connect(connect_token);

        let mut interval = tokio::time::interval(TICK_DURATION);
        let mut last_tick = Instant::now();

        loop {
            // TODO: Thoroughly test this simulator timestep
            let dt = interval.tick().await.duration_since(last_tick);
            last_tick += dt;

            match &mut self.state {
                SimulatedClientState::Disconnected => {
                    if let Some(event) = self.client.poll_connection(dt) {
                        match event {
                            ConnectionEvent::EnterGameSuccess { player_actor } => {
                                tracing::info!("successfully entered game");
                                self.state = SimulatedClientState::Connected(player_actor);
                            }
                            ConnectionEvent::Disconnected => {
                                tracing::error!("disconnected during connection phase");
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                SimulatedClientState::Connected(_entity) => {
                    let _game_events = self.client.update_game(dt);

                    let move_action = MoveAction::from_f32(
                        0.,
                        self.rng.random::<f32>(),
                        self.rng.random::<f32>(),
                    );

                    self.client.send_actions(Some(move_action), vec![]);
                }
            }
        }

        Ok(())
    }
}
