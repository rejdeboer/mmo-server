use fake::{Fake, faker::internet::en::Username};
use sqlx::postgres::PgPoolOptions;
use web_server::domain::SafePassword;

const TEST_PASSWORD: &str = "Test123!";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().expect("Failed to read .env file");
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new().connect(&db_url).await?;
    let passhash = SafePassword::parse(TEST_PASSWORD.to_string())
        .unwrap()
        .hash()
        .unwrap();

    let guild_id = sqlx::query!("INSERT INTO guilds (name) VALUES ('Testing Guild') RETURNING id;")
        .fetch_one(&pool)
        .await?;

    for i in 0..2 {
        let username: String = Username().fake();
        let email = format!("user{i}@test.com");
        let user_id = sqlx::query!(
            r#"
            INSERT INTO accounts (username, email, passhash) VALUES ($1, $2, $3)
            RETURNING id;
            "#,
            &username,
            &email,
            passhash.as_str(),
        )
        .fetch_one(&pool)
        .await?;

        let character_name: String = Username().fake();
        sqlx::query!(
            r#"
            INSERT INTO characters (name, account_id, guild_id) VALUES ($1, $2, $3)
            "#,
            &character_name,
            user_id.id,
            guild_id.id,
        )
        .execute(&pool)
        .await?;
    }
    println!("inserted fake users");
    Ok(())
}
