use anyhow::Result;
use sqlx::{Any, Pool, Postgres, Sqlite};
use uuid::Uuid;

mod compat;
pub mod postgres;
pub mod seed;
pub mod sqlite;

#[async_trait::async_trait]
pub trait Database: Send + Sync {
    async fn pool(&self) -> &Pool<Any>;

    async fn run_migrations(&self) -> Result<()>;
}

// Re-export implementations
pub use postgres::PostgresDatabase;
pub use sqlite::SqliteDatabase;

/// Backward compatibility function for existing server functions
/// that haven't been migrated to use AppState yet.
///
/// This initializes and returns a Postgres pool for legacy code.
/// New code should use `AppState::global().db.pool()` directly.
#[cfg(feature = "server")]
pub async fn pool() -> Result<&'static Pool<Postgres>, sqlx::Error> {
    compat::pool().await
}
