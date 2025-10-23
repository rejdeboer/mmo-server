use clap::Parser;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// A CLI to simulate MMO traffic
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The simulation seed to use
    #[arg(short, long)]
    seed: Option<u64>,
    #[arg(short, long, default_value_t = 10)]
    clients: usize,
    #[arg(long, default_value = "http://127.0.0.1:8032/seed")]
    seeder_endpoint: String,
    #[arg(long, default_value = "127.0.0.1")]
    server_host: String,
    #[arg(long, default_value_t = 8000)]
    server_port: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    let seed = args.seed.unwrap_or_else(|| {
        let random_seed = rand::rng().random();
        tracing::info!("no seed provided, generating: {random_seed}");
        random_seed
    });

    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    Ok(())
}
