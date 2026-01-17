# Local Development Mode Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add local development mode with SQLite, console emails, and filesystem storage to enable development without external services.

**Architecture:** Trait-based abstraction for Database, Email, and Storage services. Runtime mode detection via APP_MODE env var (defaults to production). AppState holds service implementations and provides them via Dioxus context to server functions.

**Tech Stack:** Rust, SQLx (with sqlite feature), tokio, anyhow, existing Dioxus 0.7 fullstack

---

## Task 1: Add SQLite dependency and update .gitignore

**Files:**
- Modify: `packages/api/Cargo.toml`
- Modify: `.gitignore`

**Step 1: Add SQLite feature to sqlx dependency**

Modify `packages/api/Cargo.toml`:
```toml
# Find the sqlx line and update it to:
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "macros", "migrate", "sqlite"] }
```

**Step 2: Update .gitignore**

Add to `.gitignore`:
```
# Local development mode
.dev/
*.db
*.db-shm
*.db-wal
```

**Step 3: Verify build**

Run: `cargo build -p api`
Expected: Compiles successfully with sqlite feature enabled

**Step 4: Commit**

```bash
git add packages/api/Cargo.toml .gitignore
git commit -m "feat: add SQLite support and gitignore local dev files"
```

---

## Task 2: Create configuration module with AppMode

**Files:**
- Create: `packages/api/src/config.rs`
- Modify: `packages/api/src/lib.rs`

**Step 1: Write test for AppMode detection**

Create `packages/api/src/config.rs`:
```rust
use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Local,
    Production,
}

impl AppMode {
    pub fn from_env() -> Self {
        match std::env::var("APP_MODE")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "local" => AppMode::Local,
            _ => AppMode::Production, // Default to production for safety
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_mode_defaults_to_production() {
        std::env::remove_var("APP_MODE");
        assert_eq!(AppMode::from_env(), AppMode::Production);
    }

    #[test]
    fn test_app_mode_local() {
        std::env::set_var("APP_MODE", "local");
        assert_eq!(AppMode::from_env(), AppMode::Local);
        std::env::remove_var("APP_MODE");
    }

    #[test]
    fn test_app_mode_case_insensitive() {
        std::env::set_var("APP_MODE", "LOCAL");
        assert_eq!(AppMode::from_env(), AppMode::Local);
        std::env::remove_var("APP_MODE");
    }

    #[test]
    fn test_app_mode_invalid_defaults_to_production() {
        std::env::set_var("APP_MODE", "invalid");
        assert_eq!(AppMode::from_env(), AppMode::Production);
        std::env::remove_var("APP_MODE");
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p api app_mode`
Expected: 4 tests pass

**Step 3: Export config module**

Add to `packages/api/src/lib.rs`:
```rust
pub mod config;
```

**Step 4: Commit**

```bash
git add packages/api/src/config.rs packages/api/src/lib.rs
git commit -m "feat: add AppMode enum with environment detection"
```

---

## Task 3: Add DatabaseConfig and AppConfig structs

**Files:**
- Modify: `packages/api/src/config.rs`

**Step 1: Add config structs**

Add to `packages/api/src/config.rs` (after AppMode impl):
```rust
#[derive(Debug, Clone)]
pub enum DatabaseConfig {
    PostgreSQL { url: String },
    SQLite { path: String },
}

#[derive(Debug, Clone)]
pub enum EmailConfig {
    SMTP {
        host: String,
        port: u16,
        username: String,
        password: String,
        from_email: String,
        from_name: String,
    },
    Console,
}

#[derive(Debug, Clone)]
pub enum StorageConfig {
    S3 {
        bucket: String,
        endpoint: String,
        region: String,
        access_key: String,
        secret_key: String,
        media_base_url: Option<String>,
    },
    Filesystem {
        base_path: String,
        serve_url: String,
    },
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub mode: AppMode,
    pub database: DatabaseConfig,
    pub email: EmailConfig,
    pub storage: StorageConfig,
    pub jwt_secret: String,
    pub app_base_url: String,
}
```

**Step 2: Add AppConfig::from_env() implementation**

Add to `packages/api/src/config.rs`:
```rust
impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let mode = AppMode::from_env();

        let jwt_secret = std::env::var("JWT_SECRET")
            .context("JWT_SECRET must be set in all modes")?;

        let (database, email, storage, app_base_url) = match mode {
            AppMode::Local => {
                let database = DatabaseConfig::SQLite {
                    path: ".dev/local.db".to_string(),
                };

                let email = EmailConfig::Console;

                let storage = StorageConfig::Filesystem {
                    base_path: ".dev/uploads".to_string(),
                    serve_url: "http://localhost:8080/dev/uploads".to_string(),
                };

                let app_base_url = std::env::var("APP_BASE_URL")
                    .unwrap_or_else(|_| "http://localhost:8080".to_string());

                (database, email, storage, app_base_url)
            }
            AppMode::Production => {
                let database = DatabaseConfig::PostgreSQL {
                    url: std::env::var("DATABASE_URL")
                        .context("DATABASE_URL required in production mode")?,
                };

                let email = EmailConfig::SMTP {
                    host: std::env::var("SMTP_HOST")
                        .context("SMTP_HOST required in production mode")?,
                    port: std::env::var("SMTP_PORT")
                        .context("SMTP_PORT required in production mode")?
                        .parse()
                        .context("SMTP_PORT must be a valid port number")?,
                    username: std::env::var("SMTP_USERNAME")
                        .context("SMTP_USERNAME required in production mode")?,
                    password: std::env::var("SMTP_PASSWORD")
                        .context("SMTP_PASSWORD required in production mode")?,
                    from_email: std::env::var("SMTP_FROM_EMAIL")
                        .context("SMTP_FROM_EMAIL required in production mode")?,
                    from_name: std::env::var("SMTP_FROM_NAME")
                        .unwrap_or_else(|_| "Alelysee".to_string()),
                };

                let storage = StorageConfig::S3 {
                    bucket: std::env::var("STORAGE_BUCKET")
                        .context("STORAGE_BUCKET required in production mode")?,
                    endpoint: std::env::var("STORAGE_ENDPOINT")
                        .context("STORAGE_ENDPOINT required in production mode")?,
                    region: std::env::var("STORAGE_REGION")
                        .context("STORAGE_REGION required in production mode")?,
                    access_key: std::env::var("STORAGE_ACCESS_KEY")
                        .context("STORAGE_ACCESS_KEY required in production mode")?,
                    secret_key: std::env::var("STORAGE_SECRET_KEY")
                        .context("STORAGE_SECRET_KEY required in production mode")?,
                    media_base_url: std::env::var("MEDIA_BASE_URL").ok(),
                };

                let app_base_url = std::env::var("APP_BASE_URL")
                    .context("APP_BASE_URL required in production mode")?;

                (database, email, storage, app_base_url)
            }
        };

        Ok(Self {
            mode,
            database,
            email,
            storage,
            jwt_secret,
            app_base_url,
        })
    }
}
```

**Step 3: Verify build**

Run: `cargo build -p api`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add packages/api/src/config.rs
git commit -m "feat: add AppConfig with environment-based initialization"
```

---

## Task 4: Create database abstraction trait

**Files:**
- Create: `packages/api/src/db/mod.rs`
- Modify: `packages/api/src/lib.rs`

**Step 1: Create database module with trait**

Create `packages/api/src/db/mod.rs`:
```rust
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
```

**Step 2: Export db module**

Add to `packages/api/src/lib.rs`:
```rust
pub mod db;
```

**Step 3: Add async_trait dependency**

Add to `packages/api/Cargo.toml`:
```toml
async-trait = "0.1"
```

**Step 4: Verify build**

Run: `cargo build -p api`
Expected: Build fails with "cannot find module postgres/sqlite/seed" - this is expected

**Step 5: Commit**

```bash
git add packages/api/src/db/mod.rs packages/api/src/lib.rs packages/api/Cargo.toml
git commit -m "feat: add Database trait abstraction"
```

---

## Task 5: Implement PostgreSQL database adapter

**Files:**
- Create: `packages/api/src/db/postgres.rs`

**Step 1: Implement PostgresDatabase**

Create `packages/api/src/db/postgres.rs`:
```rust
use super::Database;
use sqlx::{Pool, Postgres, Any, postgres::PgPoolOptions};
use anyhow::{Result, Context};

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
```

**Step 2: Verify build**

Run: `cargo build -p api`
Expected: Compiles successfully (sqlite and seed modules still missing)

**Step 3: Commit**

```bash
git add packages/api/src/db/postgres.rs
git commit -m "feat: implement PostgreSQL database adapter"
```

---

## Task 6: Implement SQLite database adapter

**Files:**
- Create: `packages/api/src/db/sqlite.rs`

**Step 1: Implement SqliteDatabase**

Create `packages/api/src/db/sqlite.rs`:
```rust
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
```

**Step 2: Verify build**

Run: `cargo build -p api`
Expected: Build fails - seed module doesn't exist yet

**Step 3: Commit**

```bash
git add packages/api/src/db/sqlite.rs
git commit -m "feat: implement SQLite database adapter"
```

---

## Task 7: Implement database seeding

**Files:**
- Create: `packages/api/src/db/seed.rs`

**Step 1: Implement seed function with mock data**

Create `packages/api/src/db/seed.rs`:
```rust
use sqlx::{Pool, Any};
use anyhow::{Result, Context};
use uuid::Uuid;
use chrono::Utc;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

pub async fn seed_database(pool: &Pool<Any>) -> Result<()> {
    // Hash password "Password123" for all users
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2
        .hash_password(b"Password123", &salt)
        .context("Failed to hash password")?
        .to_string();

    // Create sample users
    let user1_id = Uuid::new_v4();
    let user2_id = Uuid::new_v4();
    let user3_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO users (id, auth_subject, email, password_hash, email_verified, username, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(user1_id)
    .bind(user1_id.to_string())
    .bind("user1@local.dev")
    .bind(&password_hash)
    .bind(true)
    .bind("Alice Dupont")
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(pool)
    .await
    .context("Failed to insert user1")?;

    sqlx::query(
        "INSERT INTO users (id, auth_subject, email, password_hash, email_verified, username, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(user2_id)
    .bind(user2_id.to_string())
    .bind("user2@local.dev")
    .bind(&password_hash)
    .bind(true)
    .bind("Bernard Martin")
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(pool)
    .await
    .context("Failed to insert user2")?;

    sqlx::query(
        "INSERT INTO users (id, auth_subject, email, password_hash, email_verified, username, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(user3_id)
    .bind(user3_id.to_string())
    .bind("user3@local.dev")
    .bind(&password_hash)
    .bind(true)
    .bind("Claire Rousseau")
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(pool)
    .await
    .context("Failed to insert user3")?;

    // Create sample proposals
    let proposal_ids = create_proposals(pool, &[user1_id, user2_id, user3_id]).await?;

    // Create sample programs
    create_programs(pool, &[user1_id, user2_id], &proposal_ids).await?;

    // Create sample comments
    create_comments(pool, &[user1_id, user2_id, user3_id], &proposal_ids).await?;

    // Create sample votes
    create_votes(pool, &[user1_id, user2_id, user3_id], &proposal_ids).await?;

    tracing::info!("✓ Seeded local database with mock data");
    tracing::info!("  Users: user1@local.dev, user2@local.dev, user3@local.dev");
    tracing::info!("  Password (all): Password123");
    tracing::info!("  Proposals: {} | Programs: 2 | Comments: ~20", proposal_ids.len());

    Ok(())
}

async fn create_proposals(pool: &Pool<Any>, user_ids: &[Uuid]) -> Result<Vec<Uuid>> {
    let proposals = vec![
        ("Transition énergétique", "Proposer un plan de transition vers 100% d'énergies renouvelables d'ici 2035", user_ids[0]),
        ("Éducation gratuite", "Rendre l'éducation supérieure gratuite pour tous les citoyens", user_ids[1]),
        ("Transport public", "Développer un réseau de transport public accessible et gratuit", user_ids[0]),
        ("Santé universelle", "Renforcer le système de santé publique avec plus de moyens", user_ids[2]),
        ("Agriculture locale", "Soutenir l'agriculture locale et biologique", user_ids[1]),
        ("Logement social", "Construire 100,000 logements sociaux par an", user_ids[2]),
        ("Démocratie participative", "Instaurer des référendums d'initiative citoyenne", user_ids[0]),
        ("Écologie urbaine", "Créer des espaces verts dans toutes les villes", user_ids[1]),
        ("Travail et emploi", "Réduire le temps de travail à 32 heures par semaine", user_ids[2]),
        ("Culture accessible", "Rendre la culture accessible à tous avec des prix réduits", user_ids[0]),
    ];

    let mut proposal_ids = Vec::new();
    for (i, (title, description, author_id)) in proposals.iter().enumerate() {
        let id = Uuid::new_v4();
        let created_at = Utc::now() - chrono::Duration::days((proposals.len() - i) as i64);

        sqlx::query(
            "INSERT INTO proposals (id, author_id, title, description, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(id)
        .bind(author_id)
        .bind(title)
        .bind(description)
        .bind(created_at)
        .bind(created_at)
        .execute(pool)
        .await
        .context("Failed to insert proposal")?;

        proposal_ids.push(id);
    }

    Ok(proposal_ids)
}

async fn create_programs(pool: &Pool<Any>, user_ids: &[Uuid], proposal_ids: &[Uuid]) -> Result<()> {
    let program1_id = Uuid::new_v4();
    let program2_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO programs (id, author_id, title, description, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(program1_id)
    .bind(user_ids[0])
    .bind("Programme écologique")
    .bind("Un ensemble de propositions pour une transition écologique et sociale")
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(pool)
    .await
    .context("Failed to insert program1")?;

    sqlx::query(
        "INSERT INTO programs (id, author_id, title, description, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(program2_id)
    .bind(user_ids[1])
    .bind("Programme social")
    .bind("Des mesures pour plus de justice sociale et d'égalité")
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(pool)
    .await
    .context("Failed to insert program2")?;

    // Link proposals to programs
    for (i, proposal_id) in proposal_ids.iter().take(5).enumerate() {
        sqlx::query(
            "INSERT INTO program_proposals (program_id, proposal_id, position)
             VALUES ($1, $2, $3)"
        )
        .bind(program1_id)
        .bind(proposal_id)
        .bind(i as i32)
        .execute(pool)
        .await
        .ok();
    }

    Ok(())
}

async fn create_comments(pool: &Pool<Any>, user_ids: &[Uuid], proposal_ids: &[Uuid]) -> Result<()> {
    let comments = vec![
        "Excellente proposition, je soutiens totalement!",
        "Il faudrait aussi considérer l'impact économique",
        "C'est un bon début mais insuffisant à mon avis",
        "Comment financer cette mesure?",
        "Bravo pour cette initiative!",
    ];

    for proposal_id in proposal_ids.iter().take(5) {
        for (i, comment_text) in comments.iter().enumerate() {
            let author_id = user_ids[i % user_ids.len()];

            sqlx::query(
                "INSERT INTO comments (id, author_id, target_type, target_id, content, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)"
            )
            .bind(Uuid::new_v4())
            .bind(author_id)
            .bind("proposal")
            .bind(proposal_id)
            .bind(comment_text)
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(pool)
            .await
            .ok();
        }
    }

    Ok(())
}

async fn create_votes(pool: &Pool<Any>, user_ids: &[Uuid], proposal_ids: &[Uuid]) -> Result<()> {
    for proposal_id in proposal_ids {
        for (i, user_id) in user_ids.iter().enumerate() {
            let is_upvote = (i + proposal_ids.iter().position(|p| p == proposal_id).unwrap_or(0)) % 3 != 0;

            sqlx::query(
                "INSERT INTO votes (id, user_id, target_type, target_id, is_upvote, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6)"
            )
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind("proposal")
            .bind(proposal_id)
            .bind(is_upvote)
            .bind(Utc::now())
            .execute(pool)
            .await
            .ok();
        }
    }

    Ok(())
}
```

**Step 2: Verify build**

Run: `cargo build -p api`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add packages/api/src/db/seed.rs
git commit -m "feat: implement database seeding with mock French data"
```

---

## Task 8: Create email service abstraction

**Files:**
- Modify: `packages/api/src/email.rs`

**Step 1: Extract trait from existing email.rs**

Modify `packages/api/src/email.rs` to add trait:
```rust
// Add at top of file
#[async_trait::async_trait]
pub trait EmailService: Send + Sync {
    async fn send_email(&self, to: &str, subject: &str, html: &str, text: &str) -> Result<(), anyhow::Error>;
}

// Keep existing functions but we'll refactor them next
```

**Step 2: Create SMTP implementation struct**

Add to `packages/api/src/email.rs`:
```rust
pub struct SmtpEmailService {
    host: String,
    port: u16,
    username: String,
    password: String,
    from_email: String,
    from_name: String,
}

impl SmtpEmailService {
    pub fn new(host: String, port: u16, username: String, password: String, from_email: String, from_name: String) -> Self {
        Self { host, port, username, password, from_email, from_name }
    }
}

#[async_trait::async_trait]
impl EmailService for SmtpEmailService {
    async fn send_email(&self, to: &str, subject: &str, html: &str, text: &str) -> Result<(), anyhow::Error> {
        // Copy existing send_email implementation here
        use lettre::message::{header::ContentType, MultiPart, SinglePart};
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::{Message, SmtpTransport, Transport};

        let email = Message::builder()
            .from(format!("{} <{}>", self.from_name, self.from_email).parse()?)
            .to(to.parse()?)
            .subject(subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::builder().header(ContentType::TEXT_PLAIN).body(text.to_string()))
                    .singlepart(SinglePart::builder().header(ContentType::TEXT_HTML).body(html.to_string())),
            )?;

        let creds = Credentials::new(self.username.clone(), self.password.clone());

        let mailer = SmtpTransport::starttls_relay(&self.host)?
            .port(self.port)
            .credentials(creds)
            .build();

        tokio::task::spawn_blocking(move || mailer.send(&email))
            .await
            .map_err(|e| anyhow::anyhow!("Task join error: {}", e))??;

        Ok(())
    }
}
```

**Step 3: Create Console implementation**

Add to `packages/api/src/email.rs`:
```rust
pub struct ConsoleEmailService;

#[async_trait::async_trait]
impl EmailService for ConsoleEmailService {
    async fn send_email(&self, to: &str, subject: &str, html: &str, text: &str) -> Result<(), anyhow::Error> {
        println!("\n📧 EMAIL (Local Mode - Not Sent)");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("To: {}", to);
        println!("Subject: {}", subject);
        println!("────────────────────────────────");
        println!("HTML:");
        println!("{}", html);
        println!("────────────────────────────────");
        println!("Text:");
        println!("{}", text);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        Ok(())
    }
}
```

**Step 4: Update helper functions to use trait**

Modify existing helper functions to accept trait object:
```rust
pub async fn send_verification_email(
    email_service: &dyn EmailService,
    to: &str,
    token: &str,
) -> Result<(), anyhow::Error> {
    let app_base_url = std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let verification_url = format!("{}/auth/verify?token={}", app_base_url, token);

    let html = format!(
        r#"<html><body>
        <h1>Vérifiez votre adresse email</h1>
        <p>Cliquez sur le lien ci-dessous pour vérifier votre compte:</p>
        <a href="{}">{}</a>
        <p>Ce lien expire dans 24 heures.</p>
        </body></html>"#,
        verification_url, verification_url
    );

    let text = format!(
        "Vérifiez votre adresse email\n\nCliquez sur ce lien: {}\n\nCe lien expire dans 24 heures.",
        verification_url
    );

    email_service.send_email(to, "Vérifiez votre adresse email", &html, &text).await
}

pub async fn send_password_reset_email(
    email_service: &dyn EmailService,
    to: &str,
    token: &str,
) -> Result<(), anyhow::Error> {
    let app_base_url = std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let reset_url = format!("{}/auth/reset-password/confirm?token={}", app_base_url, token);

    let html = format!(
        r#"<html><body>
        <h1>Réinitialisez votre mot de passe</h1>
        <p>Cliquez sur le lien ci-dessous pour réinitialiser votre mot de passe:</p>
        <a href="{}">{}</a>
        <p>Ce lien expire dans 1 heure.</p>
        </body></html>"#,
        reset_url, reset_url
    );

    let text = format!(
        "Réinitialisez votre mot de passe\n\nCliquez sur ce lien: {}\n\nCe lien expire dans 1 heure.",
        reset_url
    );

    email_service.send_email(to, "Réinitialisez votre mot de passe", &html, &text).await
}
```

**Step 5: Verify build**

Run: `cargo build -p api`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add packages/api/src/email.rs
git commit -m "feat: refactor email service with trait abstraction"
```

---

## Task 9: Create storage service abstraction

**Files:**
- Create: `packages/api/src/storage/mod.rs`
- Create: `packages/api/src/storage/s3.rs`
- Create: `packages/api/src/storage/filesystem.rs`
- Modify: `packages/api/src/lib.rs`

**Step 1: Create storage module with trait**

Create `packages/api/src/storage/mod.rs`:
```rust
use anyhow::Result;

pub mod s3;
pub mod filesystem;

#[async_trait::async_trait]
pub trait StorageService: Send + Sync {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<()>;
    async fn get_url(&self, key: &str) -> Result<String>;
    async fn delete(&self, key: &str) -> Result<()>;
}

pub use s3::S3StorageService;
pub use filesystem::FilesystemStorageService;
```

**Step 2: Create S3 implementation stub**

Create `packages/api/src/storage/s3.rs`:
```rust
use super::StorageService;
use anyhow::Result;

pub struct S3StorageService {
    bucket: String,
    endpoint: String,
    region: String,
    access_key: String,
    secret_key: String,
    media_base_url: Option<String>,
}

impl S3StorageService {
    pub fn new(
        bucket: String,
        endpoint: String,
        region: String,
        access_key: String,
        secret_key: String,
        media_base_url: Option<String>,
    ) -> Self {
        Self {
            bucket,
            endpoint,
            region,
            access_key,
            secret_key,
            media_base_url,
        }
    }
}

#[async_trait::async_trait]
impl StorageService for S3StorageService {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<()> {
        // TODO: Implement S3 upload when needed
        // For now, this is a placeholder since storage isn't actively used yet
        tracing::warn!("S3 upload not yet implemented for key: {}", key);
        Ok(())
    }

    async fn get_url(&self, key: &str) -> Result<String> {
        if let Some(base_url) = &self.media_base_url {
            Ok(format!("{}/{}", base_url, key))
        } else {
            Ok(format!("https://{}.{}/{}", self.bucket, self.endpoint, key))
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        tracing::warn!("S3 delete not yet implemented for key: {}", key);
        Ok(())
    }
}
```

**Step 3: Create Filesystem implementation**

Create `packages/api/src/storage/filesystem.rs`:
```rust
use super::StorageService;
use anyhow::{Result, Context};
use std::path::PathBuf;

pub struct FilesystemStorageService {
    base_path: String,
    serve_url: String,
}

impl FilesystemStorageService {
    pub fn new(base_path: String, serve_url: String) -> Self {
        Self { base_path, serve_url }
    }
}

#[async_trait::async_trait]
impl StorageService for FilesystemStorageService {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<()> {
        let path = PathBuf::from(&self.base_path).join(key);

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create upload directory")?;
        }

        tokio::fs::write(&path, data)
            .await
            .context("Failed to write file")?;

        tracing::debug!("Uploaded to {}", path.display());

        Ok(())
    }

    async fn get_url(&self, key: &str) -> Result<String> {
        Ok(format!("{}/{}", self.serve_url, key))
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let path = PathBuf::from(&self.base_path).join(key);

        tokio::fs::remove_file(&path)
            .await
            .ok(); // Ignore errors if file doesn't exist

        Ok(())
    }
}
```

**Step 4: Export storage module**

Add to `packages/api/src/lib.rs`:
```rust
pub mod storage;
```

**Step 5: Verify build**

Run: `cargo build -p api`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add packages/api/src/storage/ packages/api/src/lib.rs
git commit -m "feat: add storage service abstraction (S3 and Filesystem)"
```

---

## Task 10: Create AppState with service initialization

**Files:**
- Create: `packages/api/src/state.rs`
- Modify: `packages/api/src/lib.rs`

**Step 1: Implement AppState**

Create `packages/api/src/state.rs`:
```rust
use crate::config::{AppConfig, AppMode, DatabaseConfig, EmailConfig, StorageConfig};
use crate::db::{Database, PostgresDatabase, SqliteDatabase};
use crate::email::{EmailService, SmtpEmailService, ConsoleEmailService};
use crate::storage::{StorageService, S3StorageService, FilesystemStorageService};
use anyhow::Result;
use std::sync::Arc;

pub struct AppState {
    pub db: Arc<dyn Database>,
    pub email: Arc<dyn EmailService>,
    pub storage: Arc<dyn StorageService>,
    pub config: AppConfig,
}

impl AppState {
    pub async fn from_config(config: AppConfig) -> Result<Self> {
        // Log mode
        match config.mode {
            AppMode::Local => {
                tracing::info!("🔧 App Mode: LOCAL");
                tracing::info!("   Database: SQLite (.dev/local.db)");
                tracing::info!("   Email: Console (not sending)");
                tracing::info!("   Storage: Filesystem (.dev/uploads/)");
            }
            AppMode::Production => {
                tracing::info!("🚀 App Mode: PRODUCTION");
                tracing::info!("   Database: PostgreSQL");
                tracing::info!("   Email: SMTP");
                tracing::info!("   Storage: S3");
            }
        }

        // Initialize database
        let db: Arc<dyn Database> = match &config.database {
            DatabaseConfig::PostgreSQL { url } => {
                let pg = PostgresDatabase::connect(url).await?;
                pg.run_migrations().await?;
                Arc::new(pg)
            }
            DatabaseConfig::SQLite { path } => {
                let sqlite = SqliteDatabase::connect(path).await?;
                sqlite.run_migrations().await?;
                sqlite.seed_if_empty().await?;
                Arc::new(sqlite)
            }
        };

        // Initialize email service
        let email: Arc<dyn EmailService> = match &config.email {
            EmailConfig::SMTP {
                host,
                port,
                username,
                password,
                from_email,
                from_name,
            } => Arc::new(SmtpEmailService::new(
                host.clone(),
                *port,
                username.clone(),
                password.clone(),
                from_email.clone(),
                from_name.clone(),
            )),
            EmailConfig::Console => Arc::new(ConsoleEmailService),
        };

        // Initialize storage service
        let storage: Arc<dyn StorageService> = match &config.storage {
            StorageConfig::S3 {
                bucket,
                endpoint,
                region,
                access_key,
                secret_key,
                media_base_url,
            } => Arc::new(S3StorageService::new(
                bucket.clone(),
                endpoint.clone(),
                region.clone(),
                access_key.clone(),
                secret_key.clone(),
                media_base_url.clone(),
            )),
            StorageConfig::Filesystem { base_path, serve_url } => {
                Arc::new(FilesystemStorageService::new(base_path.clone(), serve_url.clone()))
            }
        };

        Ok(Self {
            db,
            email,
            storage,
            config,
        })
    }
}
```

**Step 2: Export state module**

Add to `packages/api/src/lib.rs`:
```rust
pub mod state;
```

**Step 3: Verify build**

Run: `cargo build -p api`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add packages/api/src/state.rs packages/api/src/lib.rs
git commit -m "feat: add AppState with service initialization"
```

---

## Task 11: Update web server to use AppState

**Files:**
- Modify: `packages/web/src/main.rs`

**Step 1: Initialize AppState in main.rs**

Find the main function in `packages/web/src/main.rs` and update it:
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = api::config::AppConfig::from_env()?;

    // Initialize application state
    let state = api::state::AppState::from_config(config).await?;

    // Store state in a way accessible to server functions
    // This depends on how Dioxus fullstack provides context
    // For now, we'll use a static Arc with OnceCell

    use std::sync::OnceLock;
    static APP_STATE: OnceLock<Arc<api::state::AppState>> = OnceLock::new();
    APP_STATE.set(Arc::new(state)).expect("Failed to set AppState");

    // Continue with existing Dioxus server setup
    // ... existing code ...

    Ok(())
}
```

**Step 2: Add helper to get AppState in server functions**

Add to `packages/api/src/state.rs`:
```rust
use std::sync::{Arc, OnceLock};

static GLOBAL_STATE: OnceLock<Arc<AppState>> = OnceLock::new();

impl AppState {
    pub fn set_global(state: Arc<AppState>) {
        GLOBAL_STATE.set(state).expect("AppState already set");
    }

    pub fn global() -> Arc<AppState> {
        GLOBAL_STATE.get().expect("AppState not initialized").clone()
    }
}
```

**Step 3: Update web main.rs to use new helper**

```rust
// In main function, replace the static setup with:
api::state::AppState::set_global(Arc::new(state));
```

**Step 4: Verify build**

Run: `cargo build -p web`
Expected: May have compilation errors depending on existing main.rs structure - this is expected and will be fixed in integration

**Step 5: Commit**

```bash
git add packages/web/src/main.rs packages/api/src/state.rs
git commit -m "feat: integrate AppState into web server startup"
```

---

## Task 12: Update signup function to use AppState

**Files:**
- Modify: `packages/api/src/auth.rs`

**Step 1: Update signup function**

Find the `signup` function and update it to use AppState:
```rust
#[server]
pub async fn signup(email: String, password: String) -> Result<(), ServerFnError> {
    use crate::state::AppState;

    let state = AppState::global();
    let pool = state.db.pool().await;

    // Validate email format
    if !email.contains('@') {
        return Err(ServerFnError::ServerError("Invalid email format".to_string()));
    }

    // Validate password strength
    validate_password(&password).map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    // Check if email already exists
    let existing: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    if existing.is_some() {
        return Err(ServerFnError::ServerError("Email already registered".to_string()));
    }

    // Hash password
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ServerFnError::ServerError(format!("Password hashing failed: {}", e)))?
        .to_string();

    // Create user
    let user_id = Uuid::new_v4();
    let auth_subject = user_id.to_string();

    sqlx::query(
        "INSERT INTO users (id, auth_subject, email, password_hash, email_verified, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, NOW(), NOW())"
    )
    .bind(user_id)
    .bind(&auth_subject)
    .bind(&email)
    .bind(&password_hash)
    .bind(false)
    .execute(pool)
    .await
    .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    // Generate verification token
    let token = crate::email::generate_token();
    let token_hash = crate::email::hash_token(&token);
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

    sqlx::query(
        "INSERT INTO email_verifications (user_id, token_hash, expires_at)
         VALUES ($1, $2, $3)"
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    // Send verification email using AppState email service
    crate::email::send_verification_email(state.email.as_ref(), &email, &token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to send verification email: {}", e);
            ServerFnError::ServerError("Failed to send verification email".to_string())
        })?;

    Ok(())
}
```

**Step 2: Verify build**

Run: `cargo build -p api`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add packages/api/src/auth.rs
git commit -m "feat: update signup to use AppState email service"
```

---

## Task 13: Update request_password_reset to use AppState

**Files:**
- Modify: `packages/api/src/auth.rs`

**Step 1: Update request_password_reset function**

Find and update the `request_password_reset` function:
```rust
#[server]
pub async fn request_password_reset(email: String) -> Result<(), ServerFnError> {
    use crate::state::AppState;

    let state = AppState::global();
    let pool = state.db.pool().await;

    // Look up user by email
    let user: Option<(Uuid, Option<String>)> = sqlx::query_as(
        "SELECT id, password_hash FROM users WHERE email = $1"
    )
    .bind(&email)
    .fetch_optional(pool)
    .await
    .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    // Always return success for security (don't reveal if email exists)
    if let Some((user_id, password_hash)) = user {
        // Only send reset email if user has a password (not OAuth-only)
        if password_hash.is_some() {
            // Generate reset token
            let token = crate::email::generate_token();
            let token_hash = crate::email::hash_token(&token);
            let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

            sqlx::query(
                "INSERT INTO password_resets (user_id, token_hash, expires_at)
                 VALUES ($1, $2, $3)"
            )
            .bind(user_id)
            .bind(&token_hash)
            .bind(expires_at)
            .execute(pool)
            .await
            .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

            // Send reset email using AppState email service
            crate::email::send_password_reset_email(state.email.as_ref(), &email, &token)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to send password reset email: {}", e);
                    // Don't reveal failure to user for security
                })?;
        }
    }

    Ok(())
}
```

**Step 2: Verify build**

Run: `cargo build -p api`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add packages/api/src/auth.rs
git commit -m "feat: update request_password_reset to use AppState"
```

---

## Task 14: Add static file serving for local mode uploads

**Files:**
- Modify: `packages/web/src/main.rs`

**Step 1: Add tower-http dependency**

Add to `packages/web/Cargo.toml`:
```toml
tower-http = { version = "0.6", features = ["fs"] }
tower = "0.5"
```

**Step 2: Add static file serving route**

In `packages/web/src/main.rs`, add static file serving for local mode:
```rust
// After initializing AppState, before starting server:
use tower_http::services::ServeDir;
use tower::ServiceBuilder;

// If in local mode, serve .dev/uploads
if matches!(state.config.mode, api::config::AppMode::Local) {
    tracing::info!("   Serving local uploads at /dev/uploads");

    // Add route to serve .dev/uploads directory
    // This is framework-specific - adapt to how Dioxus fullstack handles static routes
    // The exact implementation depends on the server setup in main.rs
}
```

**Step 3: Document that this needs framework-specific implementation**

Add comment:
```rust
// TODO: Integrate ServeDir with Dioxus fullstack router
// The exact approach depends on how the app currently serves static files
// May need to use axum Router::nest_service if using axum backend
```

**Step 4: Commit**

```bash
git add packages/web/Cargo.toml packages/web/src/main.rs
git commit -m "feat: add static file serving for local uploads (needs integration)"
```

---

## Task 15: Update env.example with APP_MODE

**Files:**
- Modify: `env.example`

**Step 1: Add APP_MODE documentation**

Update `env.example`:
```bash
## App Mode (optional, defaults to production)
# Set to "local" for development without external services (SQLite, console emails, filesystem storage)
# Leave unset or set to "production" for Railway deployment
# APP_MODE=local

## Required in all modes
JWT_SECRET=your-secret-key-min-32-chars

## Required in production mode only (not needed if APP_MODE=local)
DATABASE_URL=postgres://postgres:postgres@localhost:5432/alelysee

# Auth (OIDC/OAuth)
AUTH_AUTHORIZE_URL=https://auth.example.com/oauth2/authorize
AUTH_CLIENT_ID=your-client-id
AUTH_REDIRECT_URI=http://localhost:8080/auth/callback
AUTH_ISSUER=https://auth.example.com/
AUTH_JWKS_URL=https://auth.example.com/.well-known/jwks.json

# SMTP (for email/password auth)
SMTP_HOST=your-smtp-host
SMTP_PORT=587
SMTP_USERNAME=your-smtp-username
SMTP_PASSWORD=your-smtp-password
SMTP_FROM_EMAIL=noreply@yourdomain.com
SMTP_FROM_NAME=Alelysee

# Application URLs
APP_BASE_URL=http://localhost:8080

# Object storage uploads
STORAGE_BUCKET=your-storage-bucket
STORAGE_ENDPOINT=https://storage.example.com
STORAGE_REGION=auto
STORAGE_ACCESS_KEY=your-access-key
STORAGE_SECRET_KEY=your-secret-key

# Optional: for video playback (recommended)
MEDIA_BASE_URL=
```

**Step 2: Commit**

```bash
git add env.example
git commit -m "docs: update env.example with APP_MODE documentation"
```

---

## Task 16: Update README with local development documentation

**Files:**
- Modify: `README.md`

**Step 1: Add Local Development section**

Add after "Quick Start" section in `README.md`:
```markdown
## Local Development

### Quick Start (No External Services)

Run the app locally without PostgreSQL, SMTP, or S3:

1. **Set up local mode:**
   ```bash
   cp env.example .env
   # Edit .env and set:
   # APP_MODE=local
   # JWT_SECRET=dev-secret-min-32-chars-for-local-testing
   ```

2. **Run the app:**
   ```bash
   make dev
   ```

3. **Login with mock user:**
   - Email: `user1@local.dev`
   - Password: `Password123`

### Local Mode Features

- **SQLite database** - Data stored in `.dev/local.db`
- **Pre-seeded mock data** - 3 users, 10 proposals, 2 programs, ~20 comments
- **Console emails** - Email content printed to stdout (not sent)
- **Filesystem uploads** - Videos stored in `.dev/uploads/`
- **No external services required** - No PostgreSQL, SMTP, or S3

### Mock User Accounts

All users have the same password: `Password123`

- `user1@local.dev` - Alice Dupont
- `user2@local.dev` - Bernard Martin
- `user3@local.dev` - Claire Rousseau

### Resetting Local Data

Delete the local database to start fresh:
```bash
rm -rf .dev/
make dev  # Restarts with fresh seed data
```

### Production Mode Locally

To test with real services locally:
```bash
# Remove APP_MODE=local from .env or set APP_MODE=production
# Ensure all production env vars are set (DATABASE_URL, SMTP_*, etc.)
make dev
```
```

**Step 2: Add troubleshooting section**

Add to README:
```markdown
### Local Mode Troubleshooting

**Database locked errors:**
- SQLite doesn't handle many concurrent writes well
- Use PostgreSQL locally for heavy concurrent testing
- Or reduce concurrent operations

**Emails not visible:**
- Check stdout/console for email output
- Should see "📧 EMAIL (Local Mode - Not Sent)"

**Video uploads not working:**
- Check `.dev/uploads/` directory exists and is writable
- Check console for error messages
- Directory is auto-created on first upload

**Mock users can't login:**
- Verify credentials: `user1@local.dev` / `Password123`
- Check startup logs for "✓ Seeded local database"
- Delete `.dev/local.db` and restart to re-seed
```

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add comprehensive local development documentation"
```

---

## Task 17: Create example .env for local development

**Files:**
- Create: `.env.local.example`

**Step 1: Create local example env file**

Create `.env.local.example`:
```bash
# Local Development Mode Configuration
# Copy this file to .env for local development without external services

# Enable local mode (SQLite, console emails, filesystem storage)
APP_MODE=local

# Required: JWT secret for authentication tokens
# Generate with: openssl rand -base64 32
JWT_SECRET=dev-secret-min-32-chars-for-local-testing-only

# Optional: Override default app URL (defaults to http://localhost:8080)
# APP_BASE_URL=http://localhost:8080

# ============================================================================
# The following are NOT needed in local mode (services are mocked):
# ============================================================================

# DATABASE_URL - Uses SQLite at .dev/local.db instead
# SMTP_* - Emails printed to console instead
# STORAGE_* - Files stored in .dev/uploads/ instead
```

**Step 2: Commit**

```bash
git add .env.local.example
git commit -m "docs: add .env.local.example for easy local setup"
```

---

## Task 18: Add logging for mode detection

**Files:**
- Modify: `packages/api/src/state.rs`

**Step 1: Enhance logging in AppState initialization**

Update the logging in `AppState::from_config`:
```rust
// After creating services, before returning Ok(Self):
tracing::info!("✅ Application initialized successfully");
match config.mode {
    AppMode::Local => {
        tracing::info!("📝 Local development credentials:");
        tracing::info!("   user1@local.dev / Password123");
        tracing::info!("   user2@local.dev / Password123");
        tracing::info!("   user3@local.dev / Password123");
    }
    AppMode::Production => {
        tracing::info!("🔒 Production mode active - all services connected");
    }
}
```

**Step 2: Verify build**

Run: `cargo build -p api`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add packages/api/src/state.rs
git commit -m "feat: add helpful logging for mode detection and credentials"
```

---

## Task 19: Test local mode end-to-end

**Files:**
- None (testing task)

**Step 1: Set up local environment**

```bash
cp .env.local.example .env
```

**Step 2: Run the application**

```bash
cargo run -p web
```

Expected output:
```
🔧 App Mode: LOCAL
   Database: SQLite (.dev/local.db)
   Email: Console (not sending)
   Storage: Filesystem (.dev/uploads/)
INFO  [api::db] Running migrations...
INFO  [api::db] ✓ Seeded local database with mock data
      Users: user1@local.dev, user2@local.dev, user3@local.dev
      Password (all): Password123
      Proposals: 10 | Programs: 2 | Comments: ~20
✅ Application initialized successfully
📝 Local development credentials:
   user1@local.dev / Password123
   user2@local.dev / Password123
   user3@local.dev / Password123
```

**Step 3: Test signup flow**

Navigate to http://localhost:8080/auth/signup and create a new account.

Expected: Console should print email verification message with token

**Step 4: Test signin with mock user**

Navigate to http://localhost:8080/auth/signin

Use: `user1@local.dev` / `Password123`

Expected: Successful login

**Step 5: Verify database persistence**

Stop and restart the server. Login should still work with same credentials.

**Step 6: Document any issues found**

Note: This is a manual testing step. Document any issues in commit message.

**Step 7: Commit**

```bash
git commit --allow-empty -m "test: verify local mode end-to-end functionality"
```

---

## Task 20: Test production mode configuration validation

**Files:**
- None (testing task)

**Step 1: Test missing DATABASE_URL in production**

```bash
# Remove .env or set APP_MODE=production without DATABASE_URL
unset APP_MODE
unset DATABASE_URL
cargo run -p web
```

Expected: Error message "DATABASE_URL required in production mode"

**Step 2: Test missing SMTP config in production**

```bash
export DATABASE_URL=postgres://test
unset SMTP_HOST
cargo run -p web
```

Expected: Error message "SMTP_HOST required in production mode"

**Step 3: Test defaults to production mode**

```bash
unset APP_MODE  # Should default to production
cargo run -p web
```

Expected: Production mode, requires all production env vars

**Step 4: Document validation works correctly**

**Step 5: Commit**

```bash
git commit --allow-empty -m "test: verify production mode validation"
```

---

## Task 21: Update desktop and mobile main.rs

**Files:**
- Modify: `packages/desktop/src/main.rs`
- Modify: `packages/mobile/src/main.rs`

**Step 1: Update desktop main.rs**

Apply same AppState initialization as web:
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = api::config::AppConfig::from_env()?;
    let state = api::state::AppState::from_config(config).await?;
    api::state::AppState::set_global(Arc::new(state));

    // ... existing desktop app launch code ...

    Ok(())
}
```

**Step 2: Update mobile main.rs**

Apply same AppState initialization:
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = api::config::AppConfig::from_env()?;
    let state = api::state::AppState::from_config(config).await?;
    api::state::AppState::set_global(Arc::new(state));

    // ... existing mobile app launch code ...

    Ok(())
}
```

**Step 3: Verify build**

Run: `cargo build -p desktop && cargo build -p mobile`
Expected: Both compile successfully

**Step 4: Commit**

```bash
git add packages/desktop/src/main.rs packages/mobile/src/main.rs
git commit -m "feat: add AppState initialization to desktop and mobile"
```

---

## Task 22: Final build and formatting

**Files:**
- All

**Step 1: Run full workspace build**

```bash
cargo build --workspace --release
```

Expected: All packages compile successfully

**Step 2: Run all tests**

```bash
cargo test --workspace
```

Expected: All tests pass (including new AppMode tests)

**Step 3: Format code**

```bash
cargo fmt --all
```

**Step 4: Run clippy**

```bash
cargo clippy --workspace -- -D warnings
```

Expected: No warnings

**Step 5: Commit formatting**

```bash
git add -A
git commit -m "chore: format code and fix clippy warnings"
```

---

## Task 23: Create migration guide document

**Files:**
- Create: `docs/LOCAL_MODE_MIGRATION.md`

**Step 1: Create migration guide**

Create `docs/LOCAL_MODE_MIGRATION.md`:
```markdown
# Local Development Mode Migration Guide

This document explains how to migrate your development setup to use the new local mode.

## For Developers

### Before (Required PostgreSQL, SMTP, S3)

```bash
# Set up all services
docker-compose up -d postgres
# Configure SMTP credentials
# Configure S3 credentials
# Set all env vars in .env

make dev
```

### After (No External Services)

```bash
# Just set two env vars
cp .env.local.example .env
# Edit .env:
#   APP_MODE=local
#   JWT_SECRET=dev-secret-here

make dev
```

### What Changed

1. **Database**: PostgreSQL → SQLite (`.dev/local.db`)
2. **Emails**: SMTP → Console output
3. **Storage**: S3 → Filesystem (`.dev/uploads/`)
4. **Config**: Many env vars → Just `APP_MODE=local` and `JWT_SECRET`

### Mock Data

Three users are pre-seeded:
- user1@local.dev / Password123
- user2@local.dev / Password123
- user3@local.dev / Password123

Plus 10 proposals, 2 programs, and ~20 comments in French.

### Resetting Data

```bash
rm -rf .dev/
make dev  # Fresh seed
```

## For Production Deployments

### Railway

**No changes needed!**

The app defaults to production mode when `APP_MODE` is not set.

### Environment Variables

Production still requires:
- DATABASE_URL
- SMTP_HOST, SMTP_PORT, etc.
- STORAGE_BUCKET, STORAGE_ENDPOINT, etc.
- JWT_SECRET
- APP_BASE_URL

### Opting Into Local Mode

Only set `APP_MODE=local` if you explicitly want local mode (e.g., staging environment).

## Troubleshooting

### "JWT_SECRET must be set"

Set `JWT_SECRET` in `.env` - it's required in all modes.

### "DATABASE_URL required in production mode"

Either:
1. Set `APP_MODE=local` in `.env` for local development
2. Set `DATABASE_URL` to use production mode with PostgreSQL

### Mock users can't login

Delete `.dev/` directory and restart - database will re-seed.

### Emails not showing

Check console/stdout for "📧 EMAIL (Local Mode - Not Sent)" messages.
```

**Step 2: Commit**

```bash
git add docs/LOCAL_MODE_MIGRATION.md
git commit -m "docs: add local mode migration guide"
```

---

## Summary

This implementation plan adds local development mode with:

1. **Configuration module** - AppMode enum, AppConfig with environment-based initialization
2. **Database abstraction** - Trait with PostgreSQL and SQLite implementations, auto-seeding
3. **Email abstraction** - Trait with SMTP and Console implementations
4. **Storage abstraction** - Trait with S3 and Filesystem implementations
5. **Application state** - Centralized service initialization and dependency injection
6. **Updated server functions** - Use AppState instead of direct database connections
7. **Comprehensive documentation** - README, env.example, migration guide
8. **Testing** - Manual E2E tests for both local and production modes

**Total Tasks**: 23
**Estimated Time**: 4-6 hours (with testing)

**After completion**:
- Developers can run app with just `APP_MODE=local` and `JWT_SECRET`
- Production deployments unchanged (default to production mode)
- All services abstracted for future flexibility (easy to swap implementations)
