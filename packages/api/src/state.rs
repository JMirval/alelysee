use crate::config::{AppConfig, AppMode, DatabaseConfig, EmailConfig, StorageConfig};
use crate::db::{Database, PostgresDatabase, SqliteDatabase};
use crate::email::{ConsoleEmailService, EmailService, SmtpEmailService};
use crate::storage::{filesystem::FilesystemStorageService, s3::S3StorageService, StorageService};
use anyhow::Result;
use std::sync::{Arc, OnceLock};

/// Global application state containing all service implementations
pub struct AppState {
    pub db: Arc<dyn Database>,
    pub email: Arc<dyn EmailService>,
    pub storage: Arc<dyn StorageService>,
    pub config: AppConfig,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl AppState {
    /// Create AppState from configuration
    ///
    /// This initializes all services based on the mode (Local vs Production)
    /// and handles migrations and seeding for SQLite databases.
    pub async fn from_config(config: AppConfig) -> Result<Self> {
        // Required for sqlx::Any pools; without this, AnyPoolOptions panics at runtime.
        sqlx::any::install_default_drivers();

        // Log the mode we're running in
        match config.mode {
            AppMode::Local => tracing::info!("🔧 App Mode: LOCAL"),
            AppMode::Production => tracing::info!("🚀 App Mode: PRODUCTION"),
        }

        match &config.database {
            DatabaseConfig::PostgreSQL { .. } => tracing::info!("   Database: PostgreSQL"),
            DatabaseConfig::SQLite { path } => tracing::info!("   Database: SQLite ({})", path),
        }
        match &config.email {
            EmailConfig::SMTP { .. } => tracing::info!("   Email: SMTP"),
            EmailConfig::Console => tracing::info!("   Email: Console (not sending)"),
        }
        match &config.storage {
            StorageConfig::S3 { .. } => tracing::info!("   Storage: S3-compatible"),
            StorageConfig::Filesystem { base_path, .. } => {
                tracing::info!("   Storage: Filesystem ({})", base_path)
            }
        }

        // Initialize database
        let db: Arc<dyn Database> = match &config.database {
            DatabaseConfig::PostgreSQL { url } => {
                tracing::info!("Connecting to PostgreSQL...");
                let postgres = PostgresDatabase::connect(url).await?;
                postgres.run_migrations().await?;
                tracing::info!("✓ PostgreSQL connected and migrations applied");
                Arc::new(postgres)
            }
            DatabaseConfig::SQLite { path } => {
                tracing::info!("Connecting to SQLite: {}", path);

                // Ensure .dev directory exists
                if let Some(parent) = std::path::Path::new(path).parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let sqlite = SqliteDatabase::connect(path).await?;
                sqlite.run_migrations().await?;
                tracing::info!("✓ SQLite connected and migrations applied");

                // Check if we need to seed (only for local mode)
                let pool = sqlite.pool().await;
                let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0);

                if count == 0 {
                    tracing::info!("Seeding empty database with mock data...");
                    crate::db::seed::seed_database(pool).await?;
                    tracing::info!("✓ Database seeded successfully");
                    tracing::info!(
                        "  Mock users: user1@local.dev, user2@local.dev, user3@local.dev"
                    );
                    tracing::info!("  Password (all): Password123");
                }

                Arc::new(sqlite)
            }
        };

        // Initialize email service
        let email: Arc<dyn EmailService> = match &config.email {
            EmailConfig::SMTP { .. } => {
                tracing::info!("Using SMTP email service");
                Arc::new(SmtpEmailService)
            }
            EmailConfig::Console => {
                tracing::info!("Using Console email service (local mode)");
                Arc::new(ConsoleEmailService)
            }
        };

        // Initialize storage service
        let storage: Arc<dyn StorageService> = match &config.storage {
            StorageConfig::S3 { bucket, .. } => {
                tracing::info!("Using S3 storage: bucket={}", bucket);
                // Note: S3StorageService is currently a stub implementation
                Arc::new(S3StorageService::new())
            }
            StorageConfig::Filesystem {
                base_path,
                serve_url,
            } => {
                tracing::info!("Using Filesystem storage: {}", base_path);

                // Ensure uploads directory exists
                std::fs::create_dir_all(base_path)?;

                Arc::new(FilesystemStorageService::new(base_path, serve_url))
            }
        };

        let state = Self {
            db,
            email,
            storage,
            config,
        };

        // Log final mode summary
        match state.config.mode {
            AppMode::Local => {
                tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                tracing::info!("🔧 LOCAL MODE ACTIVE");
                tracing::info!("   No external dependencies required");
                tracing::info!("   Database: .dev/local.db");
                tracing::info!("   Uploads: .dev/uploads/");
                tracing::info!("   Email: console output only");
                tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            }
            AppMode::Production => {
                tracing::info!("✓ Production mode initialized");
            }
        }

        Ok(state)
    }

    /// Set the global AppState instance
    ///
    /// This should be called once at server startup.
    /// Panics if called more than once.
    pub fn set_global(state: Arc<Self>) {
        STATE
            .set(state)
            .expect("AppState::set_global called more than once");
    }

    /// Get the global AppState instance
    ///
    /// Panics if called before set_global.
    pub fn global() -> Arc<Self> {
        // In tests, check thread-local state first
        #[cfg(feature = "server")]
        {
            if let Some(test_state) = TEST_STATE.with(|s| s.borrow().clone()) {
                return test_state;
            }
        }

        STATE
            .get()
            .expect("AppState::global called before set_global")
            .clone()
    }
}

/// Global state storage using OnceLock for thread-safe initialization
pub(crate) static STATE: OnceLock<Arc<AppState>> = OnceLock::new();

#[cfg(feature = "server")]
thread_local! {
    /// Thread-local state override for testing
    pub(crate) static TEST_STATE: std::cell::RefCell<Option<Arc<AppState>>> = const { std::cell::RefCell::new(None) };
}
