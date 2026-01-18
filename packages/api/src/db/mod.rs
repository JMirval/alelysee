use crate::config::DatabaseConfig;
use anyhow::Result;
use sqlx::{Any, Pool, Postgres};
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

#[cfg(feature = "server")]
pub fn uuid_to_db(value: Uuid) -> String {
    value.to_string()
}

#[cfg(feature = "server")]
pub fn uuid_from_db(value: &str) -> Result<Uuid, dioxus::prelude::ServerFnError> {
    Uuid::parse_str(value)
        .map_err(|_| dioxus::prelude::ServerFnError::new("invalid uuid from database"))
}

#[cfg(feature = "server")]
pub fn datetime_from_db(
    value: &str,
) -> Result<time::OffsetDateTime, dioxus::prelude::ServerFnError> {
    use time::macros::format_description;

    if let Ok(dt) =
        time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
    {
        return Ok(dt);
    }

    let fmt_with_offset = format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second][offset_hour sign:mandatory][offset_minute]"
    );
    if let Ok(dt) = time::OffsetDateTime::parse(value, &fmt_with_offset) {
        return Ok(dt);
    }

    let fmt_with_offset_colon = format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]"
    );
    if let Ok(dt) = time::OffsetDateTime::parse(value, &fmt_with_offset_colon) {
        return Ok(dt);
    }

    let fmt_no_offset = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    if let Ok(dt) = time::PrimitiveDateTime::parse(value, &fmt_no_offset) {
        return Ok(dt.assume_utc());
    }

    Err(dioxus::prelude::ServerFnError::new(
        "invalid timestamp from database",
    ))
}

#[cfg(feature = "server")]
pub fn tags_to_db(tags: &[String]) -> Result<String, dioxus::prelude::ServerFnError> {
    serde_json::to_string(tags)
        .map_err(|_| dioxus::prelude::ServerFnError::new("invalid tags for database"))
}

#[cfg(feature = "server")]
pub fn tags_from_db(value: &str) -> Result<Vec<String>, dioxus::prelude::ServerFnError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if trimmed.starts_with('[') {
        return serde_json::from_str(trimmed)
            .map_err(|_| dioxus::prelude::ServerFnError::new("invalid tags from database"));
    }

    Ok(trimmed
        .split(',')
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect())
}

#[cfg(feature = "server")]
pub fn is_sqlite() -> bool {
    matches!(
        crate::state::AppState::global().config.database,
        DatabaseConfig::SQLite { .. }
    )
}

/// Backward compatibility function for existing server functions
/// that haven't been migrated to use AppState yet.
///
/// This initializes and returns a Postgres pool for legacy code.
/// New code should use `AppState::global().db.pool()` directly.
#[cfg(feature = "server")]
pub async fn pool() -> Result<&'static Pool<Postgres>, sqlx::Error> {
    compat::pool().await
}
