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

#[cfg(feature = "server")]
pub fn load_dotenv() {
    use std::path::Path;

    // Try current working directory first
    let _ = dotenvy::dotenv();

    // Also try the workspace root (two levels above this crate)
    let workspace_env = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(".env");
    if workspace_env.exists() {
        let _ = dotenvy::from_path(workspace_env);
    }
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let workspace_root = workspace_root.canonicalize().unwrap_or(workspace_root);
        let mode = AppMode::from_env();

        // JWT_SECRET is required in all modes
        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET environment variable is required".to_string())?;

        let app_base_url =
            std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

        let (database, email, storage) = match mode {
            AppMode::Local => {
                // Local mode: use SQLite, Console email, Filesystem storage
                let database_path = std::env::var("LOCAL_DB_PATH").unwrap_or_else(|_| {
                    workspace_root
                        .join(".dev/local.db")
                        .to_string_lossy()
                        .to_string()
                });
                let database = DatabaseConfig::SQLite {
                    path: database_path,
                };

                let email = EmailConfig::Console;

                let storage = StorageConfig::Filesystem {
                    base_path: workspace_root
                        .join(".dev/uploads")
                        .to_string_lossy()
                        .to_string(),
                    serve_url: "http://localhost:8080/dev/uploads".to_string(),
                };

                (database, email, storage)
            }
            AppMode::Production => {
                // Production mode: validate all required env vars
                let database_url = std::env::var("DATABASE_URL")
                    .map_err(|_| "DATABASE_URL is required in production mode".to_string())?;
                let database = DatabaseConfig::PostgreSQL { url: database_url };

                let smtp_host = std::env::var("SMTP_HOST")
                    .map_err(|_| "SMTP_HOST is required in production mode".to_string())?;
                let smtp_port = std::env::var("SMTP_PORT")
                    .map_err(|_| "SMTP_PORT is required in production mode".to_string())?
                    .parse::<u16>()
                    .map_err(|_| "SMTP_PORT must be a valid port number".to_string())?;
                let smtp_username = std::env::var("SMTP_USERNAME")
                    .map_err(|_| "SMTP_USERNAME is required in production mode".to_string())?;
                let smtp_password = std::env::var("SMTP_PASSWORD")
                    .map_err(|_| "SMTP_PASSWORD is required in production mode".to_string())?;
                let smtp_from_email = std::env::var("SMTP_FROM_EMAIL")
                    .map_err(|_| "SMTP_FROM_EMAIL is required in production mode".to_string())?;
                let smtp_from_name =
                    std::env::var("SMTP_FROM_NAME").unwrap_or_else(|_| "Heliastes".to_string());

                let email = EmailConfig::SMTP {
                    host: smtp_host,
                    port: smtp_port,
                    username: smtp_username,
                    password: smtp_password,
                    from_email: smtp_from_email,
                    from_name: smtp_from_name,
                };

                let bucket = std::env::var("STORAGE_BUCKET")
                    .map_err(|_| "STORAGE_BUCKET is required in production mode".to_string())?;
                let endpoint = std::env::var("STORAGE_ENDPOINT")
                    .map_err(|_| "STORAGE_ENDPOINT is required in production mode".to_string())?;
                let region = std::env::var("STORAGE_REGION")
                    .map_err(|_| "STORAGE_REGION is required in production mode".to_string())?;
                let access_key = std::env::var("STORAGE_ACCESS_KEY")
                    .map_err(|_| "STORAGE_ACCESS_KEY is required in production mode".to_string())?;
                let secret_key = std::env::var("STORAGE_SECRET_KEY")
                    .map_err(|_| "STORAGE_SECRET_KEY is required in production mode".to_string())?;
                let media_base_url = std::env::var("MEDIA_BASE_URL").ok();

                let storage = StorageConfig::S3 {
                    bucket,
                    endpoint,
                    region,
                    access_key,
                    secret_key,
                    media_base_url,
                };

                (database, email, storage)
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
