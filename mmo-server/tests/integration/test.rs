use crate::helpers::spawn_app;
use bevy::prelude::*;

#[test]
fn testing() {
    let app = spawn_app();
    let (client, mut transport) = app.create_client();

    let condition = |world: &mut World| -> bool { true };

    app.run_app_until_condition_or_timeout(condition);
}
