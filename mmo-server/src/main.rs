use mmo_server::{application, configuration};

fn main() -> std::io::Result<()> {
    // TODO: Use custom logger?
    // let subscriber = get_subscriber();
    // subscriber.init();

    let settings = configuration::get_configuration().expect("config fetched");

    let (mut app, _) = application::build(settings)?;
    bevy::log::info!("starting application");
    app.run();
    Ok(())
}
