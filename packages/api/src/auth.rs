use crate::types::{Profile, User};
use dioxus::prelude::ServerFnError;
use uuid::Uuid;

#[cfg(feature = "server")]
use sqlx::Row;

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
    use tracing::{debug, info};

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
        debug!("auth.jwk_set: fetching jwks");

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

        match header.alg {
            Algorithm::RS256 => {
                debug!("auth.verify_id_token: alg=RS256");
                // OAuth flow - existing verification
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

                let token =
                    decode::<Claims>(id_token, &key, &validation).context("jwt verify failed")?;
                Ok(token.claims.sub)
            }
            Algorithm::HS256 => {
                debug!("auth.verify_id_token: alg=HS256");
                // Local email/password flow - new verification
                let user_id = verify_local_jwt(id_token)?;
                Ok(user_id.to_string())
            }
            _ => Err(anyhow!("unsupported jwt algorithm: {:?}", header.alg)),
        }
    }

    pub async fn ensure_user_for_subject(subject: &str) -> Result<User, ServerFnError> {
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        // Try fetch existing
        if let Some(row) = sqlx::query(
            "select CAST(id as TEXT) as id, CAST(created_at as TEXT) as created_at from users where auth_subject = $1",
        )
            .bind(subject)
            .fetch_optional(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
        {
            let id = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
            let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
            debug!("auth.ensure_user_for_subject: existing user_id={}", id);
            return Ok(User { id, created_at });
        }

        // Create
        let row = sqlx::query(
            "insert into users (auth_subject) values ($1) returning CAST(id as TEXT) as id, CAST(created_at as TEXT) as created_at",
        )
        .bind(subject)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let id = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
        info!("auth.ensure_user_for_subject: created user_id={}", id);
        Ok(User {
            id,
            created_at: crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?,
        })
    }

    pub async fn get_profile_for_user(user_id: Uuid) -> Result<Option<Profile>, ServerFnError> {
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let row = sqlx::query(
            "select CAST(user_id as TEXT) as user_id, display_name, bio, avatar_url, location, CAST(updated_at as TEXT) as updated_at from profiles where user_id = $1",
        )
        .bind(crate::db::uuid_to_db(user_id))
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        if let Some(row) = row {
            let profile = Profile {
                user_id: crate::db::uuid_from_db(&row.get::<String, _>("user_id"))?,
                display_name: row.get("display_name"),
                bio: row.get("bio"),
                avatar_url: row.get("avatar_url"),
                location: row.get("location"),
                updated_at: crate::db::datetime_from_db(&row.get::<String, _>("updated_at"))?,
            };
            debug!("auth.get_profile_for_user: hit user_id={}", user_id);
            Ok(Some(profile))
        } else {
            debug!("auth.get_profile_for_user: miss user_id={}", user_id);
            Ok(None)
        }
    }

    pub fn validate_password(password: &str) -> Result<(), anyhow::Error> {
        if password.len() < 8 {
            return Err(anyhow::anyhow!("Password must be at least 8 characters"));
        }
        if !password.chars().any(|c| c.is_uppercase()) {
            return Err(anyhow::anyhow!(
                "Password must contain at least one uppercase letter"
            ));
        }
        if !password.chars().any(|c| c.is_lowercase()) {
            return Err(anyhow::anyhow!(
                "Password must contain at least one lowercase letter"
            ));
        }
        if !password.chars().any(|c| c.is_numeric()) {
            return Err(anyhow::anyhow!("Password must contain at least one number"));
        }
        Ok(())
    }

    use jsonwebtoken::{encode, EncodingKey, Header};

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct LocalClaims {
        sub: String,
        iss: String,
        exp: usize,
        iat: usize,
    }

    pub fn generate_local_jwt(user_id: Uuid) -> Result<String, anyhow::Error> {
        let secret = std::env::var("JWT_SECRET").context("JWT_SECRET must be set")?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as usize;

        let exp = now + (30 * 24 * 60 * 60); // 30 days

        let claims = LocalClaims {
            sub: user_id.to_string(),
            iss: "alelysee".to_string(),
            exp,
            iat: now,
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )?;

        debug!("auth.generate_local_jwt: user_id={}", user_id);
        Ok(token)
    }

    pub fn verify_local_jwt(token: &str) -> Result<Uuid, anyhow::Error> {
        let secret = std::env::var("JWT_SECRET").context("JWT_SECRET must be set")?;

        let mut validation = jsonwebtoken::Validation::new(Algorithm::HS256);
        validation.set_issuer(&["alelysee"]);

        let token_data = jsonwebtoken::decode::<LocalClaims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        )?;

        let user_id = Uuid::parse_str(&token_data.claims.sub)?;
        debug!("auth.verify_local_jwt: user_id={}", user_id);
        Ok(user_id)
    }

    pub fn email_domain(email: &str) -> &str {
        email.split('@').nth(1).unwrap_or("invalid")
    }

    pub fn email_label(email: &str) -> String {
        format!("{} (len={})", email_domain(email), email.len())
    }

    #[cfg(test)]
    mod password_tests {
        use super::*;

        #[test]
        fn test_validate_password_accepts_strong_password() {
            assert!(validate_password("Passw0rd").is_ok());
            assert!(validate_password("MyP@ssw0rd123").is_ok());
        }

        #[test]
        fn test_validate_password_rejects_short() {
            let result = validate_password("Pass1");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("8 characters"));
        }

        #[test]
        fn test_validate_password_rejects_no_uppercase() {
            let result = validate_password("password1");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("uppercase"));
        }

        #[test]
        fn test_validate_password_rejects_no_lowercase() {
            let result = validate_password("PASSWORD1");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("lowercase"));
        }

        #[test]
        fn test_validate_password_rejects_no_number() {
            let result = validate_password("Password");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("number"));
        }
    }

    #[cfg(test)]
    mod jwt_tests {
        use super::*;

        #[tokio::test]
        async fn test_generate_and_verify_local_jwt() {
            std::env::set_var("JWT_SECRET", "test-secret-key-for-testing-32chars");

            let user_id = Uuid::new_v4();
            let token = generate_local_jwt(user_id).unwrap();

            assert!(!token.is_empty());

            let verified_id = verify_local_jwt(&token).unwrap();
            assert_eq!(verified_id, user_id);
        }

        #[tokio::test]
        async fn test_verify_local_jwt_rejects_invalid_token() {
            std::env::set_var("JWT_SECRET", "test-secret-key-for-testing-32chars");

            let result = verify_local_jwt("invalid.jwt.token");
            assert!(result.is_err());
        }
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
    #[cfg(feature = "server")]
    tracing::debug!("auth.public_config");
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
        tracing::debug!("auth.me_from_id_token: token_len={}", id_token.len());
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
        tracing::debug!("auth.require_user_id: token_len={}", id_token.len());
        let sub = server::verify_id_token(&id_token)
            .await
            .map_err(|e| ServerFnError::new(format!("auth: {e:#}")))?;
        let user = server::ensure_user_for_subject(&sub).await?;
        tracing::debug!("auth.require_user_id: user_id={}", user.id);
        Ok(user.id)
    }
}

/// Sign up a new user with email and password
#[dioxus::prelude::post("/api/auth/signup")]
pub async fn signup(email: String, password: String) -> Result<(), ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (email, password);
        Err(ServerFnError::new("signup is server-only"))
    }

    #[cfg(feature = "server")]
    {
        // Get AppState
        let state = crate::state::AppState::global();
        tracing::info!(
            "auth.request_password_reset: email={}",
            server::email_label(&email)
        );
        tracing::info!("auth.signup: email={}", server::email_label(&email));

        // Validate email format (basic check)
        if !email.contains('@') || email.len() < 3 {
            return Err(ServerFnError::new("Invalid email address"));
        }

        // Validate password
        server::validate_password(&password).map_err(|e| ServerFnError::new(e.to_string()))?;

        // Get database pool from state
        let pool = state.db.pool().await;

        // Check if email already exists (select 1 avoids UUID decoding on Postgres)
        let existing = sqlx::query("select 1 from users where email = $1")
            .bind(&email)
            .fetch_optional(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        if existing.is_some() {
            tracing::info!("auth.signup: email already registered");
            return Err(ServerFnError::new("Email already registered"));
        }

        // Hash password
        use argon2::password_hash::SaltString;
        use argon2::{Argon2, PasswordHasher};

        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| ServerFnError::new(format!("Password hashing failed: {}", e)))?
            .to_string();

        // Create user
        // Generate UUID before insert
        let user_id = Uuid::new_v4();
        let auth_subject = user_id.to_string();

        let insert_user_sql = if crate::db::is_sqlite() {
            "insert into users (id, email, password_hash, auth_subject) values ($1, $2, $3, $4)"
        } else {
            "insert into users (id, email, password_hash, auth_subject) values ($1::uuid, $2, $3, $4)"
        };

        sqlx::query(insert_user_sql)
            .bind(user_id.to_string())
            .bind(&email)
            .bind(&password_hash)
            .bind(&auth_subject)
            .execute(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        tracing::info!("auth.signup: user created user_id={}", user_id);

        // Generate verification token
        let token = crate::email::generate_token();
        let token_hash = crate::email::hash_token(&token);

        // Calculate expiration; use native timestamptz on Postgres, RFC3339 text on SQLite.
        let expires_at = time::OffsetDateTime::now_utc() + time::Duration::hours(24);
        let expires_at_str = expires_at
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| ServerFnError::new(format!("Failed to format timestamp: {}", e)))?;

        if crate::db::is_sqlite() {
            sqlx::query(
                "insert into email_verifications (user_id, token_hash, expires_at) values ($1, $2, $3)",
            )
            .bind(user_id.to_string())
            .bind(&token_hash)
            .bind(&expires_at_str)
            .execute(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        } else {
            sqlx::query(
                "insert into email_verifications (user_id, token_hash, expires_at) values ($1::uuid, $2, $3::timestamptz)",
            )
            .bind(user_id.to_string())
            .bind(&token_hash)
            .bind(&expires_at_str)
            .execute(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        // Send verification email using the email service from state
        crate::email::send_verification_email(state.email.as_ref(), &email, &token)
            .await
            .map_err(|e| {
                tracing::warn!("auth.signup: failed to send verification email: {}", e);
                ServerFnError::new("Failed to send verification email")
            })?;

        tracing::info!("auth.signup: verification email queued");
        Ok(())
    }
}

/// Verify email address with token
#[dioxus::prelude::post("/api/auth/verify-email")]
pub async fn verify_email(token: String) -> Result<(), ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = token;
        Err(ServerFnError::new("verify_email is server-only"))
    }

    #[cfg(feature = "server")]
    {
        tracing::info!("auth.verify_email: token_len={}", token.len());
        let token_hash = crate::email::hash_token(&token);
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        // Look up verification token
        let verification = sqlx::query(
            "select CAST(user_id as TEXT) as user_id, CAST(expires_at as TEXT) as expires_at from email_verifications where token_hash = $1",
        )
        .bind(&token_hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let verification = verification
            .ok_or_else(|| ServerFnError::new("Verification link is invalid or has expired"))?;

        let user_id = crate::db::uuid_from_db(&verification.get::<String, _>("user_id"))?;
        let expires_at = crate::db::datetime_from_db(&verification.get::<String, _>("expires_at"))?;

        // Check expiration
        if time::OffsetDateTime::now_utc() > expires_at {
            tracing::info!("auth.verify_email: token expired");
            return Err(ServerFnError::new("Verification link has expired"));
        }

        // Mark email as verified
        sqlx::query("update users set email_verified = true where id = $1")
            .bind(crate::db::uuid_to_db(user_id))
            .execute(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        // Delete verification token
        sqlx::query("delete from email_verifications where token_hash = $1")
            .bind(&token_hash)
            .execute(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        tracing::info!("auth.verify_email: verified user_id={}", user_id);
        Ok(())
    }
}

/// Sign in with email and password
#[dioxus::prelude::post("/api/auth/signin")]
pub async fn signin(email: String, password: String) -> Result<String, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (email, password);
        Err(ServerFnError::new("signin is server-only"))
    }

    #[cfg(feature = "server")]
    {
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;
        tracing::info!("auth.signin: email={}", server::email_label(&email));

        // Look up user by email
        let user = sqlx::query(
            "select CAST(id as TEXT) as id, password_hash, email_verified from users where email = $1",
        )
        .bind(&email)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let user = user.ok_or_else(|| ServerFnError::new("Invalid email or password"))?;

        let user_id = crate::db::uuid_from_db(&user.get::<String, _>("id"))?;
        let password_hash: Option<String> = user.get("password_hash");
        // SQLite stores booleans as integers (0 = false, 1 = true)
        let email_verified: bool = match user.try_get::<bool, _>("email_verified") {
            Ok(v) => v,
            Err(_) => {
                // Fallback for SQLite: treat integer as boolean
                let v: i64 = user.get("email_verified");
                v != 0
            }
        };

        // Check if user has password (not OAuth-only)
        let password_hash = password_hash.ok_or_else(|| {
            ServerFnError::new("This account uses OAuth. Please sign in with your provider.")
        })?;

        // Verify password
        use argon2::{Argon2, PasswordHash, PasswordVerifier};

        let parsed_hash = PasswordHash::new(&password_hash)
            .map_err(|e| ServerFnError::new(format!("Invalid password hash: {}", e)))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| ServerFnError::new("Invalid email or password"))?;

        // Check email verified
        if !email_verified {
            tracing::info!("auth.signin: email not verified");
            return Err(ServerFnError::new(
                "Please verify your email before signing in",
            ));
        }

        // Generate JWT
        let token = server::generate_local_jwt(user_id)
            .map_err(|e| ServerFnError::new(format!("Failed to generate token: {}", e)))?;

        tracing::info!("auth.signin: success user_id={}", user_id);
        Ok(token)
    }
}

/// Resend verification email (always returns success for security)
#[dioxus::prelude::post("/api/auth/resend-verification")]
pub async fn resend_verification_email(email: String) -> Result<(), ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = email;
        Err(ServerFnError::new(
            "resend_verification_email is server-only",
        ))
    }

    #[cfg(feature = "server")]
    {
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;
        tracing::info!(
            "auth.resend_verification_email: email={}",
            server::email_label(&email)
        );

        let user = sqlx::query(
            "select CAST(id as TEXT) as id, email_verified, password_hash from users where email = $1",
        )
        .bind(&email)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        if let Some(user) = user {
            let user_id = crate::db::uuid_from_db(&user.get::<String, _>("id"))?;
            let password_hash: Option<String> = user.get("password_hash");
            let email_verified: bool = match user.try_get::<bool, _>("email_verified") {
                Ok(v) => v,
                Err(_) => {
                    let v: i64 = user.get("email_verified");
                    v != 0
                }
            };

            if !email_verified && password_hash.is_some() {
                let token = crate::email::generate_token();
                let token_hash = crate::email::hash_token(&token);
                let expires_at = time::OffsetDateTime::now_utc() + time::Duration::hours(24);
                let expires_at_str = expires_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .map_err(|e| {
                        ServerFnError::new(format!("Failed to format timestamp: {}", e))
                    })?;

                let insert = if crate::db::is_sqlite() {
                    sqlx::query(
                        "insert into email_verifications (user_id, token_hash, expires_at) values ($1, $2, $3)",
                    )
                    .bind(user_id.to_string())
                    .bind(&token_hash)
                    .bind(&expires_at_str)
                } else {
                    sqlx::query(
                        "insert into email_verifications (user_id, token_hash, expires_at) values ($1::uuid, $2, $3::timestamptz)",
                    )
                    .bind(user_id.to_string())
                    .bind(&token_hash)
                    .bind(&expires_at_str)
                };

                if let Err(e) = insert.execute(pool).await {
                    tracing::warn!("auth.resend_verification_email: store token failed: {}", e);
                } else if let Err(e) =
                    crate::email::send_verification_email(state.email.as_ref(), &email, &token)
                        .await
                {
                    tracing::warn!("auth.resend_verification_email: send email failed: {}", e);
                } else {
                    tracing::info!("auth.resend_verification_email: dispatched");
                }
            }
        } else {
            tracing::debug!("auth.resend_verification_email: user not found");
        }

        Ok(())
    }
}

/// Request password reset (always returns success for security)
#[dioxus::prelude::post("/api/auth/request-password-reset")]
pub async fn request_password_reset(email: String) -> Result<(), ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = email;
        Err(ServerFnError::new("request_password_reset is server-only"))
    }

    #[cfg(feature = "server")]
    {
        // Get AppState
        let state = crate::state::AppState::global();

        // Get database pool from state
        let pool = state.db.pool().await;

        // Look up user by email
        // sqlx::Any will automatically convert UUID columns to strings
        let user_lookup_sql = if crate::db::is_sqlite() {
            "select id, password_hash from users where email = $1"
        } else {
            "select CAST(id as TEXT) as id, password_hash from users where email = $1"
        };

        let user = sqlx::query(user_lookup_sql)
            .bind(&email)
            .fetch_optional(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        // If user exists and has password_hash, send reset email
        if let Some(user) = user {
            let user_id_str: String = user.get("id");
            let password_hash: Option<String> = user.get("password_hash");

            // Only send if user has a password (not OAuth-only)
            if password_hash.is_some() {
                // Generate reset token
                let token = crate::email::generate_token();
                let token_hash = crate::email::hash_token(&token);

                let expires_at = time::OffsetDateTime::now_utc() + time::Duration::hours(1);
                let expires_at_str = expires_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .map_err(|e| {
                        eprintln!("Failed to format timestamp: {}", e);
                        e
                    })
                    .ok();

                let insert_reset = if crate::db::is_sqlite() {
                    expires_at_str.as_ref().map(|expires_str| {
                        sqlx::query(
                            "insert into password_resets (user_id, token_hash, expires_at) values ($1, $2, $3)",
                        )
                        .bind(&user_id_str)
                        .bind(&token_hash)
                        .bind(expires_str)
                    })
                } else {
                    expires_at_str.map(|expires_str| {
                        sqlx::query(
                            "insert into password_resets (user_id, token_hash, expires_at) values ($1::uuid, $2, $3::timestamptz)",
                        )
                        .bind(&user_id_str)
                        .bind(&token_hash)
                        .bind(expires_str)
                    })
                };

                if let Some(query) = insert_reset {
                    if let Err(e) = query.execute(pool).await {
                        tracing::warn!("auth.request_password_reset: store token failed: {}", e);
                    } else if let Err(e) = crate::email::send_password_reset_email(
                        state.email.as_ref(),
                        &email,
                        &token,
                    )
                    .await
                    {
                        tracing::warn!("auth.request_password_reset: send email failed: {}", e);
                    }
                }
            }
            tracing::info!(
                "auth.request_password_reset: dispatched user_id={}",
                user_id_str
            );
        } else {
            tracing::debug!("auth.request_password_reset: user not found");
        }

        // Always return success (security: don't reveal if email exists)
        Ok(())
    }
}

/// Reset password with token
#[dioxus::prelude::post("/api/auth/reset-password")]
pub async fn reset_password(token: String, new_password: String) -> Result<(), ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (token, new_password);
        Err(ServerFnError::new("reset_password is server-only"))
    }

    #[cfg(feature = "server")]
    {
        tracing::info!("auth.reset_password: token_len={}", token.len());
        // Validate new password
        server::validate_password(&new_password).map_err(|e| ServerFnError::new(e.to_string()))?;

        let token_hash = crate::email::hash_token(&token);
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        // Look up reset token
        let reset = sqlx::query(
            "select CAST(user_id as TEXT) as user_id, CAST(expires_at as TEXT) as expires_at from password_resets where token_hash = $1",
        )
        .bind(&token_hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let reset =
            reset.ok_or_else(|| ServerFnError::new("Reset link is invalid or has expired"))?;

        let user_id = crate::db::uuid_from_db(&reset.get::<String, _>("user_id"))?;
        let expires_at = crate::db::datetime_from_db(&reset.get::<String, _>("expires_at"))?;

        // Check expiration
        if time::OffsetDateTime::now_utc() > expires_at {
            tracing::info!("auth.reset_password: token expired");
            return Err(ServerFnError::new("Reset link has expired"));
        }

        // Hash new password
        use argon2::password_hash::SaltString;
        use argon2::{Argon2, PasswordHasher};

        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = argon2
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(|e| ServerFnError::new(format!("Password hashing failed: {}", e)))?
            .to_string();

        // Update password
        sqlx::query("update users set password_hash = $1 where id = $2")
            .bind(&password_hash)
            .bind(crate::db::uuid_to_db(user_id))
            .execute(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        // Delete reset token
        sqlx::query("delete from password_resets where token_hash = $1")
            .bind(&token_hash)
            .execute(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        tracing::info!("auth.reset_password: success user_id={}", user_id);
        Ok(())
    }
}
