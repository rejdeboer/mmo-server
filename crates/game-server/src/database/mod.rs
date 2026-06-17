mod character;

use crate::configuration::Settings;
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
pub use character::*;
use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Resource, Clone)]
pub struct DatabasePool(pub PgPool);

pub struct DatabasePlugin;

impl Plugin for DatabasePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_database_pool);
    }
}

pub fn get_connection_pool(settings: &Settings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(settings.database.with_db())
}

fn setup_database_pool(
    mut commands: Commands,
    runtime: Res<TokioTasksRuntime>,
    settings: Res<Settings>,
) {
    let pool = runtime
        .runtime()
        .block_on(async move { get_connection_pool(&settings) });
    commands.insert_resource(DatabasePool(pool));
}
