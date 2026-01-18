use crate::config::{AppConfig, AppMode};
use crate::db::sqlite::SqliteDatabase;
use crate::db::Database;
use crate::email::ConsoleEmailService;
use crate::state::AppState;
use crate::storage::filesystem::FilesystemStorageService;
use sqlx::{Any, Pool};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

pub struct TestContext {
    pub pool: Pool<Any>,
    pub state: Arc<AppState>,
    db_path: PathBuf,
    uploads_path: PathBuf,
}

impl TestContext {
    pub async fn new() -> Self {
        // Install sqlx drivers for Any pool
        sqlx::any::install_default_drivers();

        let test_id = Uuid::new_v4();
        let db_path = PathBuf::from(format!(".test-{}.db", test_id));
        let uploads_path = PathBuf::from(format!(".test-uploads-{}", test_id));

        // Set local mode
        std::env::set_var("APP_MODE", "local");
        std::env::set_var("JWT_SECRET", "test-secret-key-min-32-characters-long");
        std::env::set_var("APP_BASE_URL", "http://localhost:8080");

        // Create SQLite database
        let database = SqliteDatabase::connect(&db_path.to_string_lossy())
            .await
            .expect("Failed to create test database");

        // Run migrations
        database
            .run_migrations()
            .await
            .expect("Failed to run migrations");

        // Get pool
        let pool = database.pool().await.clone();

        // Create AppState
        let config = AppConfig {
            mode: AppMode::Local,
            database: crate::config::DatabaseConfig::SQLite {
                path: db_path.to_string_lossy().to_string(),
            },
            email: crate::config::EmailConfig::Console,
            storage: crate::config::StorageConfig::Filesystem {
                base_path: uploads_path.to_string_lossy().to_string(),
                serve_url: "http://localhost:8080/dev/uploads".to_string(),
            },
            jwt_secret: "test-secret-key-min-32-characters-long".to_string(),
            app_base_url: "http://localhost:8080".to_string(),
        };

        let state = Arc::new(AppState {
            db: Arc::new(database),
            email: Arc::new(ConsoleEmailService),
            storage: Arc::new(FilesystemStorageService::new(
                &uploads_path.to_string_lossy(),
                "http://localhost:8080/dev/uploads",
            )),
            config: config.clone(),
        });

        Self {
            pool,
            state,
            db_path,
            uploads_path,
        }
    }

    pub fn set_global(&self) {
        AppState::set_global(self.state.clone());
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        // Cleanup test database and uploads
        let _ = std::fs::remove_file(&self.db_path);
        let _ = std::fs::remove_dir_all(&self.uploads_path);
    }
}
