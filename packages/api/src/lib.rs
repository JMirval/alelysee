//! This crate contains all shared fullstack server functions.
use dioxus::prelude::*;

pub mod types;

#[cfg(feature = "server")]
mod db;

#[cfg(feature = "server")]
pub(crate) use db::pool;

mod auth;
mod proposals;
mod programs;
mod votes;
mod comments;
mod profile;
mod activity;
mod uploads;

#[cfg(all(test, feature = "server"))]
mod test_support;

#[cfg(test)]
mod types_tests;

#[cfg(all(test, feature = "server"))]
mod domain_tests;

/// Echo the user input on the server.
#[post("/api/echo")]
pub async fn echo(input: String) -> Result<String, ServerFnError> {
    Ok(input)
}

#[get("/api/config")]
pub async fn public_config() -> Result<auth::PublicConfig, ServerFnError> {
    auth::public_config().await
}

#[post("/api/auth/me")]
pub async fn auth_me(id_token: String) -> Result<auth::Me, ServerFnError> {
    auth::me_from_id_token(id_token).await
}

pub use programs::ProgramDetail;
pub use programs::{add_program_item, create_program, get_program, list_programs, update_program};
pub use proposals::{create_proposal, get_proposal, list_proposals, update_proposal};
pub use votes::set_vote;
pub use comments::{create_comment, list_comments};
pub use profile::upsert_profile;
pub use activity::list_my_activity;
pub use uploads::{create_video_upload_intent, finalize_video_upload, list_videos};
