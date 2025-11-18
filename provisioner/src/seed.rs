use fake::{Fake, faker::internet::en::Username};
use sqlx::PgPool;
use web_server::domain::SafePassword;

const TEST_PASSWORD: &str = "Test123!";

pub async fn seed_db(pool: PgPool, count: usize) -> anyhow::Result<()> {
    let passhash = SafePassword::parse(TEST_PASSWORD.to_string())
        .unwrap()
        .hash()
        .unwrap();

    sqlx::query!("TRUNCATE TABLE characters, accounts, guilds RESTART IDENTITY CASCADE;")
        .execute(&pool)
        .await?;
    let guild_id = sqlx::query!("INSERT INTO guilds (name) VALUES ('Testing Guild') RETURNING id;")
        .fetch_one(&pool)
        .await?;

    for i in 0..count {
        let username: String = format!("{}{i}", Username().fake::<String>());
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
    tracing::info!(?count, "inserted fake users");
    Ok(())
}
