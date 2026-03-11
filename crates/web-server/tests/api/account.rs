use crate::helpers::spawn_app;
use web_client::WebClientError;
use web_types::AccountCreate;

#[tokio::test]
async fn create_account_success() -> Result<(), WebClientError> {
    let app = spawn_app().await;
    app.client
        .create_account(&AccountCreate {
            username: "rejdeboer".into(),
            email: "rick.deboer@live.nl".into(),
            password: "SuperSecret123!".into(),
        })
        .await?;
    Ok(())
}
