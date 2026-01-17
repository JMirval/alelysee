use sqlx::{Pool, Postgres, Sqlite, Any};
use uuid::Uuid;
use anyhow::Result;

pub mod postgres;
pub mod sqlite;
pub mod seed;

#[async_trait::async_trait]
pub trait Database: Send + Sync {
    async fn pool(&self) -> &Pool<Any>;

    async fn run_migrations(&self) -> Result<()>;
}

// Re-export implementations
pub use postgres::PostgresDatabase;
pub use sqlite::SqliteDatabase;
