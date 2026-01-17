use super::Database;
use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, Any, Pool, Postgres};

pub struct PostgresDatabase {
    pool: Pool<Any>,
}

impl PostgresDatabase {
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .context("Failed to connect to PostgreSQL")?;

        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl Database for PostgresDatabase {
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
