use super::StorageService;
use anyhow::Result;
use async_trait::async_trait;

/// S3-compatible storage service implementation (production)
pub struct S3StorageService;

impl S3StorageService {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl StorageService for S3StorageService {
    async fn upload(&self, key: &str, _data: Vec<u8>) -> Result<()> {
        tracing::warn!(
            "S3StorageService::upload not yet implemented (key: {})",
            key
        );
        Ok(())
    }

    async fn get_url(&self, key: &str) -> Result<String> {
        tracing::warn!(
            "S3StorageService::get_url not yet implemented (key: {})",
            key
        );
        Ok(format!("https://placeholder.example.com/{}", key))
    }

    async fn delete(&self, key: &str) -> Result<()> {
        tracing::warn!(
            "S3StorageService::delete not yet implemented (key: {})",
            key
        );
        Ok(())
    }
}
