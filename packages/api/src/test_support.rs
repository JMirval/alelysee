//! Test helpers for server-feature API tests.
//!
//! These tests are designed to:
//! - be **extensive** when a Postgres `DATABASE_URL` is available
//! - **skip gracefully** when no DB is configured (so CI/dev without DB still passes)

#![cfg(all(test, feature = "server"))]

use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::sync::OnceLock;
use tokio::sync::Mutex;

static POOL: OnceLock<Pool<Postgres>> = OnceLock::new();
static RESET_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn reset_lock() -> &'static Mutex<()> {
    RESET_LOCK.get_or_init(|| Mutex::new(()))
}

pub async fn pool() -> Option<&'static Pool<Postgres>> {
    if let Some(pool) = POOL.get() {
        return Some(pool);
    }

    let database_url = match std::env::var("DATABASE_URL") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return None,
    };

    // One schema for the whole test run. We reset tables between tests.
    let schema = "heliastes_test";

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .after_connect(move |conn, _meta| {
            Box::pin(async move {
                // Ensure everything (migrations, queries) happens inside the test schema.
                sqlx::query(&format!(r#"set search_path to "{schema}""#))
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&database_url)
        .await
        .ok()?;

    // Ensure schema exists
    let _ = sqlx::query(&format!(r#"create schema if not exists "{schema}""#))
        .execute(&pool)
        .await;

    // Run migrations into test schema
    if sqlx::migrate!("./migrations").run(&pool).await.is_err() {
        return None;
    }

    let _ = POOL.set(pool);
    POOL.get()
}

pub async fn reset_db() -> Option<()> {
    let pool = pool().await?;
    let _guard = reset_lock().lock().await;

    // Truncate in dependency order. RESTART IDENTITY is harmless with UUID PKs but fine.
    let _ = sqlx::query(
        r#"
        truncate table
            activity,
            votes,
            comments,
            videos,
            program_items,
            programs,
            proposals,
            profiles,
            users
        restart identity
        "#,
    )
    .execute(pool)
    .await
    .ok()?;

    Some(())
}


