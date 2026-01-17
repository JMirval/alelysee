use anyhow::Result;
use async_trait::async_trait;

pub mod filesystem;
pub mod s3;

/// Trait for storage service implementations
#[async_trait]
pub trait StorageService: Send + Sync {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<()>;
    async fn get_url(&self, key: &str) -> Result<String>;
    async fn delete(&self, key: &str) -> Result<()>;
}
