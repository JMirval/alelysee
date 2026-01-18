use crate::config::{AppConfig, AppMode};
use crate::db::sqlite::SqliteDatabase;
use crate::db::Database;
use crate::email::ConsoleEmailService;
use crate::state::AppState;
use crate::storage::filesystem::FilesystemStorageService;
use sqlx::{Any, Pool};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use uuid::Uuid;

// Global mutex to serialize test execution since AppState::set_global can only be called once
static TEST_MUTEX: Mutex<()> = Mutex::new(());

pub struct TestContext {
    pub pool: Pool<Any>,
    pub state: Arc<AppState>,
    db_path: PathBuf,
    uploads_path: PathBuf,
    _guard: MutexGuard<'static, ()>,
}

impl TestContext {
    pub async fn new() -> Self {
        // Acquire the test mutex to serialize test execution
        // This prevents multiple tests from calling AppState::set_global simultaneously
        let guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

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
            _guard: guard,
        }
    }

    pub fn set_global(&self) {
        // For tests, set thread-local state instead of global state
        // This allows each test to have its own isolated AppState
        crate::state::TEST_STATE.with(|s| {
            *s.borrow_mut() = Some(self.state.clone());
        });
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        // Clear thread-local state
        crate::state::TEST_STATE.with(|s| {
            *s.borrow_mut() = None;
        });

        // Cleanup test database and uploads
        let _ = std::fs::remove_file(&self.db_path);
        let _ = std::fs::remove_dir_all(&self.uploads_path);
    }
}
