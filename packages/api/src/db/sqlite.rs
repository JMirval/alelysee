use super::{Database, seed};
use sqlx::{Pool, Any, sqlite::SqlitePoolOptions};
use anyhow::{Result, Context};
use std::path::Path;

pub struct SqliteDatabase {
    pool: Pool<Any>,
}

impl SqliteDatabase {
    pub async fn connect(path: &str) -> Result<Self> {
        // Create .dev directory if it doesn't exist
        if let Some(parent) = Path::new(path).parent() {
            tokio::fs::create_dir_all(parent).await
                .context("Failed to create .dev directory")?;
        }

        let url = format!("sqlite:{}", path);
        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(1) // SQLite doesn't handle concurrent writes well
            .connect(&url)
            .await
            .context("Failed to connect to SQLite")?;

        Ok(Self { pool })
    }

    pub async fn seed_if_empty(&self) -> Result<()> {
        // Check if database has any users
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        if count == 0 {
            tracing::info!("Database is empty, seeding with mock data...");
            seed::seed_database(&self.pool).await?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Database for SqliteDatabase {
    async fn pool(&self) -> &Pool<Any> {
        &self.pool
    }

    async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .context("Failed to run migrations")?;
        Ok(())
    }
}
