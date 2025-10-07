use web_server::{
    configuration::{self},
    server::Application,
    telemetry::init_telemetry,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_telemetry();

    let settings = configuration::get_configuration().expect("config fetched");

    let application = Application::build(settings).await?;
    application.run_until_stopped().await?;
    Ok(())
}
