use super::StorageService;
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;

/// Filesystem storage service implementation (local development)
pub struct FilesystemStorageService {
    base_path: PathBuf,
    serve_url: String,
}

impl FilesystemStorageService {
    pub fn new(base_path: &str, serve_url: &str) -> Self {
        Self {
            base_path: PathBuf::from(base_path),
            serve_url: serve_url.to_string(),
        }
    }
}

#[async_trait]
impl StorageService for FilesystemStorageService {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<()> {
        let file_path = self.base_path.join(key);

        // Create parent directories if they don't exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Write file
        fs::write(&file_path, data).await?;

        tracing::debug!("Uploaded to {}", file_path.display());
        Ok(())
    }

    async fn get_url(&self, key: &str) -> Result<String> {
        let url = format!("{}/{}", self.serve_url.trim_end_matches('/'), key);
        tracing::debug!("Serving at {}", url);
        Ok(url)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let file_path = self.base_path.join(key);

        // Ignore error if file doesn't exist
        if file_path.exists() {
            fs::remove_file(&file_path).await?;
            tracing::debug!("Deleted {}", file_path.display());
        } else {
            tracing::debug!("File not found (already deleted): {}", file_path.display());
        }

        Ok(())
    }
}
