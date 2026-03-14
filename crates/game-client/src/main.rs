use clap::Parser;
use game_client::application::create_authenticated_app;
use game_client::configuration::get_configuration;
use game_client::decode_token;
use web_client::WebClient;

/// A dev tool to quickly spin up a testing client session
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Email of the testing client
    #[arg(short, long, default_value = "user0@test.com")]
    email: String,
    /// Password of the testing client
    #[arg(short, long, default_value = "Test123!")]
    password: String,
    /// Testing character id
    #[arg(short, long, default_value_t = 1)]
    character_id: i32,
}

async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();
    let settings = get_configuration()?;

    let rt = tokio::runtime::Runtime::new()?;
    let web_client = WebClient::new(settings.web_server.endpoint);
    let (web_client, encoded_token) = rt.block_on(async {
        tracing::info!("connecting to web server");
        web_client
            .login(&LoginBody {
                email: args.email,
                password: args.password,
            })
            .await
            .expect("logged into test user");

        let connect_token = web_client
            .select_character(args.character_id)
            .await
            .expect("selected test character");

        (web_client, connect_token)
    });
    let connect_token = decode_token(encoded_token)?;

    let mut app = create_authenticated_app(settings, web_client);
    app.run();

    Ok(())
}
