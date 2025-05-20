use crate::helpers::spawn_app;

#[tokio::test]
async fn testing() {
    let test_app = spawn_app().await;
    // TODO: Figure out a way to run integration tests for bevy
}
