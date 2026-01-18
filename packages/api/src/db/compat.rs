//! Backward compatibility layer for legacy pool() function
//!
//! This module provides the old `pool()` function that returns Pool<Postgres>
//! for functions that haven't been migrated to AppState yet.
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::sync::OnceLock;

static LEGACY_POOL: OnceLock<Pool<Postgres>> = OnceLock::new();

pub async fn pool() -> Result<&'static Pool<Postgres>, sqlx::Error> {
    if let Some(pool) = LEGACY_POOL.get() {
        return Ok(pool);
    }

    // In production mode, use DATABASE_URL
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for production mode");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let _ = LEGACY_POOL.set(pool);
    Ok(LEGACY_POOL.get().expect("pool initialized"))
}
