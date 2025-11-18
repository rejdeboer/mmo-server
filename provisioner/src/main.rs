use clap::{Parser, Subcommand, arg};
use provisioner::{Application, ServerSettings, get_configuration, init_telemetry, seed_db};
use sqlx::postgres::PgPoolOptions;
use web_server::configuration::NetcodePrivateKey;

/// A CLI to seed an MMO database
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// The URL of the DB to seed
    #[arg(global = true, long)]
    db_url: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Starts HTTP seeding server
    Serve {
        /// The port to listen on
        #[arg(long)]
        port: Option<u16>,
        /// The host to listen on
        #[arg(long)]
        host: Option<String>,
    },
    /// Seeds a given MMO DB
    Seed {
        /// The number of users to create
        #[arg(short, long, default_value_t = 2)]
        count: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_telemetry();
    let cli = Cli::parse();
    let settings = get_configuration()?;
    let pg_connect_options = match cli.db_url {
        Some(url) => url.parse()?,
        None => settings
            .database
            .expect("No CLI arg provided, so expected DB settings to be set by config file")
            .with_db(),
    };
    let pool = PgPoolOptions::new()
        .connect_with(pg_connect_options)
        .await?;

    match &cli.command {
        Commands::Seed { count } => {
            tracing::info!(?count, "inserting users");
            seed_db(pool, *count).await?;
        }
        Commands::Serve { port, host } => {
            let mut server_settings = settings.server.unwrap_or(ServerSettings {
                host: "127.0.0.1".to_string(),
                port: 8032,
                netcode_private_key: NetcodePrivateKey::default(),
            });

            if let Some(host) = host {
                server_settings.host = host.clone();
            }

            if let Some(port) = port {
                server_settings.port = *port;
            }

            let app = Application::build(server_settings, pool).await?;
            app.run_until_stopped().await?;
        }
    };

    Ok(())
}
