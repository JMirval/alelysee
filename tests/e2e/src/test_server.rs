use anyhow::Result;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use uuid::Uuid;

pub struct TestServer {
    url: String,
    process: Option<Child>,
    db_path: PathBuf,
}

impl TestServer {
    pub async fn start() -> Result<Self> {
        let port = get_random_port()?;
        let test_id = Uuid::new_v4();
        let db_path = PathBuf::from(format!(".e2e-test-{}.db", test_id));

        // Start server process with environment variables
        let process = Command::new("cargo")
            .args(&["run", "--package", "web", "--features", "server"])
            .env("APP_MODE", "local")
            .env("PORT", port.to_string())
            .env("IP", "127.0.0.1")
            .env("JWT_SECRET", "test-secret-key-min-32-characters-long")
            .env("APP_BASE_URL", format!("http://localhost:{}", port))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start server");

        let url = format!("http://localhost:{}", port);

        // Wait for server to be ready
        wait_for_server(&url).await?;

        Ok(Self {
            url,
            process: Some(process),
            db_path,
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
        let _ = std::fs::remove_file(&self.db_path);
    }
}

fn get_random_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

async fn wait_for_server(url: &str) -> Result<()> {
    // Wait up to 60 seconds for server to start (compilation + startup)
    for i in 0..600 {
        if let Ok(response) = reqwest::get(url).await {
            if response.status().is_success() {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Log progress every 5 seconds
        if i > 0 && i % 50 == 0 {
            eprintln!("Still waiting for server... ({}s)", i / 10);
        }
    }
    anyhow::bail!("Server did not start in time (waited 60s)")
}
