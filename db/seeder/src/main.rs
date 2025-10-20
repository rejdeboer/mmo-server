use db_seeder::seed::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().expect("Failed to read .env file");
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    seed(&db_url).await
}
