use web_server::routes::AccountCreate;

use crate::helpers::spawn_app;

#[tokio::test]
async fn create_account_success() {
    let app = spawn_app().await;
    let response = app
        .create_account(AccountCreate {
            username: "rejdeboer".into(),
            email: "rick.deboer@live.nl".into(),
            password: "SuperSecret123!".into(),
        })
        .await;
    assert_eq!(response.status().as_u16(), 200);
}
