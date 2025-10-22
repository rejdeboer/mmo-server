use clap::Parser;

/// A CLI to simulate MMO traffic
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The simulation seed to use
    #[arg(short, long)]
    seed: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    Ok(())
}
