#[cfg(feature = "server")]
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

#[cfg(feature = "server")]
use std::sync::OnceLock;

/// Global Postgres pool for server functions.
///
/// This is server-only. Client builds (wasm) will not include sqlx.
#[cfg(feature = "server")]
static POOL: OnceLock<Pool<Postgres>> = OnceLock::new();

#[cfg(feature = "server")]
pub async fn pool() -> Result<&'static Pool<Postgres>, sqlx::Error> {
    if let Some(pool) = POOL.get() {
        return Ok(pool);
    }

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for server builds");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    // Keep schema up-to-date in dev/prod without a separate migration step.
    // This is intentionally done once per process boot.
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Ignore if another async task initialized first; use the winner.
    let _ = POOL.set(pool);
    Ok(POOL.get().expect("pool initialized"))
}


