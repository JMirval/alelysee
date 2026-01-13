use crate::types::{Profile, User};
use dioxus::prelude::ServerFnError;
use uuid::Uuid;

#[cfg(feature = "server")]
mod server {
    use super::*;
    use anyhow::{anyhow, Context};
    use jsonwebtoken::{
        decode, decode_header,
        jwk::{AlgorithmParameters, JwkSet},
        Algorithm, DecodingKey, Validation,
    };
    use serde::Deserialize;
    use sqlx::Row;
    use std::sync::OnceLock;
    use time::OffsetDateTime;

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Claims {
        sub: String,
        iss: String,
        aud: Option<String>,
        exp: usize,
    }

    static JWK_SET: OnceLock<JwkSet> = OnceLock::new();

    async fn jwk_set() -> Result<&'static JwkSet, anyhow::Error> {
        if let Some(set) = JWK_SET.get() {
            return Ok(set);
        }

        let url = std::env::var("AUTH_JWKS_URL").context("AUTH_JWKS_URL must be set")?;

        let set: JwkSet = reqwest::Client::new()
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let _ = JWK_SET.set(set);
        Ok(JWK_SET.get().expect("jwk set initialized"))
    }

    fn expected_issuer() -> Result<String, anyhow::Error> {
        std::env::var("AUTH_ISSUER").context("AUTH_ISSUER must be set")
    }

    fn expected_audience() -> Result<String, anyhow::Error> {
        std::env::var("AUTH_CLIENT_ID").context("AUTH_CLIENT_ID must be set")
    }

    pub async fn verify_id_token(id_token: &str) -> Result<String, anyhow::Error> {
        let header = decode_header(id_token).context("invalid jwt header")?;
        let kid = header.kid.ok_or_else(|| anyhow!("jwt missing kid"))?;

        let jwks = jwk_set().await?;
        let jwk = jwks
            .keys
            .iter()
            .find(|k| k.common.key_id.as_deref() == Some(kid.as_str()))
            .ok_or_else(|| anyhow!("no matching jwk for kid"))?;

        let (n, e) = match &jwk.algorithm {
            AlgorithmParameters::RSA(rsa) => (rsa.n.clone(), rsa.e.clone()),
            _ => return Err(anyhow!("jwk is not rsa")),
        };

        let key = DecodingKey::from_rsa_components(&n, &e).context("bad rsa components")?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[expected_issuer()?]);
        validation.set_audience(&[expected_audience()?]);

        let token = decode::<Claims>(id_token, &key, &validation).context("jwt verify failed")?;
        Ok(token.claims.sub)
    }

    pub async fn ensure_user_for_subject(subject: &str) -> Result<User, ServerFnError> {
        let pool = crate::pool()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        // Try fetch existing
        if let Some(row) = sqlx::query("select id, created_at from users where auth_subject = $1")
            .bind(subject)
            .fetch_optional(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
        {
            let id: Uuid = row.get("id");
            let created_at: OffsetDateTime = row.get("created_at");
            return Ok(User { id, created_at });
        }

        // Create
        let row =
            sqlx::query("insert into users (auth_subject) values ($1) returning id, created_at")
                .bind(subject)
                .fetch_one(pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(User {
            id: row.get("id"),
            created_at: row.get("created_at"),
        })
    }

    pub async fn get_profile_for_user(user_id: Uuid) -> Result<Option<Profile>, ServerFnError> {
        let pool = crate::pool()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let row = sqlx::query(
            "select user_id, display_name, bio, avatar_url, location, updated_at from profiles where user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(row.map(|row| Profile {
            user_id: row.get("user_id"),
            display_name: row.get("display_name"),
            bio: row.get("bio"),
            avatar_url: row.get("avatar_url"),
            location: row.get("location"),
            updated_at: row.get("updated_at"),
        }))
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PublicConfig {
    pub auth_authorize_url: String,
    pub auth_client_id: String,
    pub auth_redirect_uri: String,
    pub media_base_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Me {
    pub user: User,
    pub profile: Option<Profile>,
    pub profile_complete: bool,
}

pub async fn public_config() -> Result<PublicConfig, ServerFnError> {
    let auth_authorize_url = std::env::var("AUTH_AUTHORIZE_URL")
        .map_err(|_| ServerFnError::new("AUTH_AUTHORIZE_URL not set"))?;
    let auth_client_id = std::env::var("AUTH_CLIENT_ID")
        .map_err(|_| ServerFnError::new("AUTH_CLIENT_ID not set"))?;
    let auth_redirect_uri = std::env::var("AUTH_REDIRECT_URI")
        .map_err(|_| ServerFnError::new("AUTH_REDIRECT_URI not set"))?;
    let media_base_url = std::env::var("MEDIA_BASE_URL").ok();

    Ok(PublicConfig {
        auth_authorize_url,
        auth_client_id,
        auth_redirect_uri,
        media_base_url,
    })
}

pub async fn me_from_id_token(id_token: String) -> Result<Me, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = id_token;
        Err(ServerFnError::new("me_from_id_token is server-only"))
    }

    #[cfg(feature = "server")]
    {
        let sub = server::verify_id_token(&id_token)
            .await
            .map_err(|e| ServerFnError::new(format!("auth: {e:#}")))?;

        let user = server::ensure_user_for_subject(&sub).await?;
        let profile = server::get_profile_for_user(user.id).await?;
        let profile_complete = profile
            .as_ref()
            .is_some_and(|p| !p.display_name.trim().is_empty());

        Ok(Me {
            user,
            profile,
            profile_complete,
        })
    }
}

/// Resolve an authenticated user id from an id_token.
///
/// This will also upsert the `users` record for the auth subject.
pub async fn require_user_id(id_token: String) -> Result<Uuid, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = id_token;
        Err(ServerFnError::new("require_user_id is server-only"))
    }

    #[cfg(feature = "server")]
    {
        let sub = server::verify_id_token(&id_token)
            .await
            .map_err(|e| ServerFnError::new(format!("auth: {e:#}")))?;
        let user = server::ensure_user_for_subject(&sub).await?;
        Ok(user.id)
    }
}
