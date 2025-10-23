use mmo_client::GameClient;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub struct SimulatedClient {
    client: GameClient,
    character_id: i32,
    rng: ChaCha8Rng,
}

impl SimulatedClient {
    pub fn new(character_id: i32, seed: u64) -> Self {
        let client = GameClient::default();
        let rng = ChaCha8Rng::seed_from_u64(seed);

        Self {
            client,
            character_id,
            rng,
        }
    }
}
