use mmo_server::{application, configuration};

fn main() -> std::io::Result<()> {
    let settings = configuration::get_configuration().expect("config fetched");

    let (mut app, _) = application::build(settings)?;
    bevy::log::info!("starting application");
    app.run();
    Ok(())
}
