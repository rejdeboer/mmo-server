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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let seed = args.seed.unwrap_or_else(|| {
        let random_seed = rand::rng().random();
        println!("No seed provided, using randomly generated seed: {random_seed}");
        random_seed
    });

    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    Ok(())
}
