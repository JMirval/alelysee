//! This crate contains all shared fullstack server functions.
use dioxus::prelude::*;

pub mod config;
pub mod types;

#[cfg(feature = "server")]
pub mod db;

#[cfg(feature = "server")]
pub mod email;

#[cfg(feature = "server")]
pub mod storage;

#[cfg(feature = "server")]
pub mod state;

mod activity;
mod auth;
mod comments;
mod profile;
mod programs;
mod proposals;
mod uploads;
mod votes;

#[cfg(all(test, feature = "server"))]
mod test_support;

#[cfg(test)]
mod types_tests;

#[cfg(all(test, feature = "server"))]
mod domain_tests;

#[cfg(feature = "server")]
pub mod test_utils;

/// Health check endpoint
#[get("/api/health")]
pub async fn health_check() -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    tracing::debug!("health_check");
    Ok("OK".to_string())
}

/// Detailed health check with metrics
#[get("/api/health/detailed")]
pub async fn detailed_health_check() -> Result<serde_json::Value, ServerFnError> {
    use serde_json::json;

    #[cfg(feature = "server")]
    tracing::debug!("detailed_health_check");

    // Basic health response with timestamp and version info
    let health = json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
        "uptime": std::process::id(), // Process ID as simple uptime indicator
        "checks": {
            "database": "unknown", // Could be extended to check DB connection
            "storage": "unknown"   // Could be extended to check object storage access
        }
    });

    Ok(health)
}

/// Metrics endpoint for monitoring
#[get("/api/metrics")]
pub async fn metrics_endpoint() -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    tracing::debug!("metrics_endpoint");

    // Simple metrics in Prometheus format
    let metrics = r#"# HELP alelysee_requests_total Total number of requests
# TYPE alelysee_requests_total counter
alelysee_requests_total 0

# HELP alelysee_health_status Health check status (1=healthy, 0=unhealthy)
# TYPE alelysee_health_status gauge
alelysee_health_status 1

# HELP alelysee_uptime_seconds Time since application started
# TYPE alelysee_uptime_seconds gauge
alelysee_uptime_seconds 0
"#;

    Ok(metrics.to_string())
}

/// Echo the user input on the server.
#[post("/api/echo")]
pub async fn echo(input: String) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    tracing::debug!("echo: len={}", input.len());
    Ok(input)
}

#[get("/api/config")]
pub async fn public_config() -> Result<auth::PublicConfig, ServerFnError> {
    #[cfg(feature = "server")]
    tracing::debug!("public_config");
    auth::public_config().await
}

#[post("/api/auth/me")]
pub async fn auth_me(id_token: String) -> Result<auth::Me, ServerFnError> {
    #[cfg(feature = "server")]
    tracing::debug!("auth_me: token_len={}", id_token.len());
    auth::me_from_id_token(id_token).await
}

pub use activity::list_my_activity;
pub use auth::{request_password_reset, reset_password, signin, signup, verify_email};
pub use comments::{create_comment, list_comments};
pub use profile::upsert_profile;
pub use programs::ProgramDetail;
pub use programs::{add_program_item, create_program, get_program, list_programs, update_program};
pub use proposals::{create_proposal, get_proposal, list_proposals, update_proposal};
pub use uploads::{create_video_upload_intent, finalize_video_upload, list_videos};
pub use votes::set_vote;
