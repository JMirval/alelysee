use anyhow::Result;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct TestServer {
    url: String,
    process: Option<Child>,
    db_path: PathBuf,
    log_path: PathBuf,
}

impl TestServer {
    pub async fn start() -> Result<Self> {
        let port = get_random_port()?;
        let test_id = Uuid::new_v4();
        let db_path = PathBuf::from(format!(".e2e-test-{}.db", test_id));
        let log_path = std::env::temp_dir().join(format!("e2e-server-{}.log", test_id));
        let log_file = std::fs::File::create(&log_path)?;
        let log_file_err = log_file.try_clone()?;

        // Start server process with environment variables
        let mut process = Command::new("cargo")
            .args(["run", "--package", "web", "--features", "server"])
            .env("APP_MODE", "local")
            .env("PORT", port.to_string())
            .env("IP", "127.0.0.1")
            .env("JWT_SECRET", "test-secret-key-min-32-characters-long")
            .env("APP_BASE_URL", format!("http://localhost:{}", port))
            .env("LOCAL_DB_PATH", db_path.to_string_lossy().to_string())
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(log_file_err))
            .spawn()
            .expect("Failed to start server");

        let url = format!("http://localhost:{}", port);

        // Wait for server to be ready
        if let Err(e) = wait_for_server(&mut process, &url, &log_path).await {
            // Kill the process if server failed to start
            let _ = process.kill();
            let _ = process.wait();
            return Err(e);
        }

        Ok(Self {
            url,
            process: Some(process),
            db_path,
            log_path,
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
        let _ = std::fs::remove_file(&self.log_path);
    }
}

fn get_random_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

async fn wait_for_server(process: &mut Child, url: &str, log_path: &Path) -> Result<()> {
    let timeout_secs = std::env::var("E2E_SERVER_STARTUP_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(180);
    let timeout = Duration::from_secs(timeout_secs);
    let start = Instant::now();
    let mut last_log = Instant::now();

    // Wait for server to start (compilation + startup)
    loop {
        if let Some(status) = process.try_wait()? {
            let log_tail = read_log_tail(log_path, 16 * 1024);
            anyhow::bail!(
                "Server process exited before startup (status: {status}).\n{}",
                format_log_tail(log_path, log_tail)
            );
        }

        if let Ok(response) = reqwest::get(url).await {
            if response.status().is_success() {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Log progress every 5 seconds
        if last_log.elapsed() >= Duration::from_secs(5) {
            eprintln!(
                "Still waiting for server... ({}s / {}s)",
                start.elapsed().as_secs(),
                timeout_secs
            );
            last_log = Instant::now();
        }

        if start.elapsed() >= timeout {
            let log_tail = read_log_tail(log_path, 16 * 1024);
            anyhow::bail!(
                "Server did not start in time (waited {}s).\n{}",
                timeout_secs,
                format_log_tail(log_path, log_tail)
            );
        }
    }
}

fn read_log_tail(path: &Path, max_bytes: usize) -> Option<String> {
    let contents = std::fs::read(path).ok()?;
    if contents.is_empty() {
        return None;
    }
    let start = contents.len().saturating_sub(max_bytes);
    Some(String::from_utf8_lossy(&contents[start..]).to_string())
}

fn format_log_tail(log_path: &Path, log_tail: Option<String>) -> String {
    match log_tail {
        Some(log_tail) if !log_tail.trim().is_empty() => {
            format!("Server log tail:\n{log_tail}")
        }
        _ => format!("No server logs captured. Log file: {}", log_path.display()),
    }
}
