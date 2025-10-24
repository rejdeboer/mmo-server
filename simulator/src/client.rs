use mmo_client::{ConnectionEvent, Entity, GameClient};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::time::Duration;
use tokio::time::Instant;

const TICK_DURATION: Duration = Duration::from_millis(1000 / 30);

pub enum SimulatedClientState {
    Disconnected,
    Connected(Entity),
}

pub struct SimulatedClient {
    client: GameClient,
    character_id: i32,
    rng: ChaCha8Rng,
    state: SimulatedClientState,
}

impl SimulatedClient {
    pub fn new(character_id: i32, seed: u64) -> Self {
        let client = GameClient::default();
        let rng = ChaCha8Rng::seed_from_u64(seed);

        Self {
            client,
            character_id,
            rng,
            state: SimulatedClientState::Disconnected,
        }
    }

    pub async fn run(mut self, host: String, port: u16) -> anyhow::Result<()> {
        tracing::info!(character_id = self.character_id, "starting bot");

        self.client.connect_unsecure(host, port, self.character_id);

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
                            ConnectionEvent::EnterGameSuccess { player_entity } => {
                                tracing::info!(
                                    character_id = self.character_id,
                                    "successfully entered game"
                                );
                                self.state = SimulatedClientState::Connected(player_entity);
                            }
                            ConnectionEvent::Disconnected => {
                                tracing::error!(
                                    character_id = self.character_id,
                                    "disconnected during connection phase"
                                );
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                SimulatedClientState::Connected(_entity) => {
                    let _game_events = self.client.update_game(dt);
                }
            }
        }

        Ok(())
    }
}
