use crate::helpers::spawn_app;
use bevy::prelude::*;

#[test]
fn testing() {
    let test_app = spawn_app();
    let (client, mut transport) = test_app.create_client();

    let condition = |world: &mut World| -> bool { true };

    test_app.run_app_until_condition_or_timeout(condition);
}
