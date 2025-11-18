use clap::Parser;
use futures::future::join_all;
use mmo_client::decode_token;
use provisioner::{ProvisionParams, ProvisionResult};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use simulator::SimulatedClient;
use std::time::Duration;

/// A CLI to simulate MMO traffic
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The simulation seed to use
    #[arg(short, long)]
    seed: Option<u64>,
    /// Number of clients to simulate
    #[arg(short, long, default_value_t = 10)]
    clients: usize,
    /// Endpoint of the provisioning server
    #[arg(long, default_value = "http://127.0.0.1:8032/provision")]
    provisioner_endpoint: String,
    /// Game server host
    #[arg(long, default_value = "127.0.0.1")]
    server_host: String,
    /// Game server port
    #[arg(long, default_value_t = 8000)]
    server_port: u16,
    /// Duration of the simulation in seconds
    #[arg(short, long, default_value_t = 60)]
    duration: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    let game_server_addr = format!("{}:{}", args.server_host, args.server_port);
    let result =
        execute_provision(&args.provisioner_endpoint, args.clients, game_server_addr).await?;

    let seed = args.seed.unwrap_or_else(|| {
        let random_seed = rand::rng().random();
        tracing::info!("no seed provided, generating: {random_seed}");
        random_seed
    });

    let mut main_rng = ChaCha8Rng::seed_from_u64(seed);
    let mut tasks = vec![];

    for token in result.tokens {
        let connect_token = decode_token(token).expect("token decoded");
        let bot_seed = main_rng.random();

        // WARNING: Because we're simulating, the client id equals the character ID
        // Need to be careful to maintain this invariant
        let client = SimulatedClient::new(connect_token.client_id as i32, bot_seed);

        tasks.push(tokio::spawn(client.run(connect_token)));
    }

    let timeout_duration = Duration::from_secs(args.duration);
    match tokio::time::timeout(timeout_duration, join_all(tasks)).await {
        Ok(results) => {
            tracing::info!("simulation finished naturally");
            let successful_runs = results
                .iter()
                .filter(|res| res.is_ok() && res.as_ref().unwrap().is_ok())
                .count();
            tracing::info!(
                "{successful_runs}/{} bots completed without error",
                args.clients
            );
        }
        Err(_) => {
            tracing::info!(
                "simulation ended after reaching the {} second timeout",
                args.duration
            );
        }
    }

    Ok(())
}

async fn execute_provision(
    endpoint: &str,
    clients: usize,
    game_server_addr: String,
) -> anyhow::Result<ProvisionResult> {
    let res = reqwest::Client::new()
        .post(endpoint)
        .json(&ProvisionParams {
            count: clients,
            server_addr: game_server_addr,
        })
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(res)
}
