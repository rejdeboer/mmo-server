use clap::Parser;
use db_seeder::SeedParameters;
use futures::future::join_all;
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
    /// Endpoint of the DB seeding server
    #[arg(long, default_value = "http://127.0.0.1:8032/seed")]
    seeder_endpoint: String,
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

    seed_db(&args.seeder_endpoint, args.clients).await?;

    let seed = args.seed.unwrap_or_else(|| {
        let random_seed = rand::rng().random();
        tracing::info!("no seed provided, generating: {random_seed}");
        random_seed
    });

    let mut main_rng = ChaCha8Rng::seed_from_u64(seed);
    let mut tasks = vec![];

    for i in 0..args.clients {
        let bot_seed = main_rng.random();
        let client = SimulatedClient::new(i as i32, bot_seed);
        tasks.push(tokio::spawn(
            client.run(args.server_host.clone(), args.server_port),
        ));
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

async fn seed_db(endpoint: &str, clients: usize) -> anyhow::Result<()> {
    reqwest::Client::new()
        .post(endpoint)
        .json(&SeedParameters { count: clients })
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
