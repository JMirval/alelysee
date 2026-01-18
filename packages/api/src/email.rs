use anyhow::Result;
use async_trait::async_trait;
use rand::Rng;
use sha2::{Digest, Sha256};
use tracing::{debug, info};

fn email_domain(email: &str) -> &str {
    email.split('@').nth(1).unwrap_or("invalid")
}

fn email_label(email: &str) -> String {
    format!("{} (len={})", email_domain(email), email.len())
}

/// Generate a cryptographically secure random token (64 hex chars from 32 bytes)
pub fn generate_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
}

/// Hash a token using SHA-256 (returns 64 hex chars)
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

use lettre::{
    message::{MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport, Transport,
};

/// Trait for email service implementations
#[async_trait]
pub trait EmailService: Send + Sync {
    async fn send_email(&self, to: &str, subject: &str, html: &str, text: &str) -> Result<()>;
}

/// SMTP email service implementation (production)
pub struct SmtpEmailService;

#[async_trait]
impl EmailService for SmtpEmailService {
    async fn send_email(&self, to: &str, subject: &str, html: &str, text: &str) -> Result<()> {
        debug!(
            "email.smtp.send_email: to={} subject_len={} html_len={} text_len={}",
            email_label(to),
            subject.len(),
            html.len(),
            text.len()
        );
        let smtp_host = std::env::var("SMTP_HOST")?;
        let smtp_port: u16 = std::env::var("SMTP_PORT")?.parse()?;
        let smtp_username = std::env::var("SMTP_USERNAME")?;
        let smtp_password = std::env::var("SMTP_PASSWORD")?;
        let smtp_from_email = std::env::var("SMTP_FROM_EMAIL")?;
        let smtp_from_name =
            std::env::var("SMTP_FROM_NAME").unwrap_or_else(|_| "Alelysee".to_string());

        let email = Message::builder()
            .from(format!("{} <{}>", smtp_from_name, smtp_from_email).parse()?)
            .to(to.parse()?)
            .subject(subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::plain(text.to_string()))
                    .singlepart(SinglePart::html(html.to_string())),
            )?;

        let creds = Credentials::new(smtp_username, smtp_password);
        let mailer = SmtpTransport::relay(&smtp_host)?
            .port(smtp_port)
            .credentials(creds)
            .build();

        // Wrap blocking SMTP operation in spawn_blocking
        tokio::task::spawn_blocking(move || mailer.send(&email))
            .await
            .map_err(|e| anyhow::anyhow!("Task join error: {}", e))??;

        Ok(())
    }
}

/// Console email service implementation (local development)
pub struct ConsoleEmailService;

#[async_trait]
impl EmailService for ConsoleEmailService {
    async fn send_email(&self, to: &str, subject: &str, html: &str, text: &str) -> Result<()> {
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

/// Send verification email
pub async fn send_verification_email(
    email_service: &dyn EmailService,
    to: &str,
    token: &str,
) -> Result<()> {
    info!(
        "email.send_verification_email: to={} token_len={}",
        email_label(to),
        token.len()
    );
    let base_url =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let verify_url = format!("{}/auth/verify?token={}", base_url, token);

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"></head>
<body style="font-family: sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
  <h1 style="color: #333;">Verify your email</h1>
  <p>Welcome to Alelysee! Please verify your email address by clicking the button below:</p>
  <p style="margin: 30px 0;">
    <a href="{}" style="background-color: #007bff; color: white; padding: 12px 24px; text-decoration: none; border-radius: 4px; display: inline-block;">Verify Email</a>
  </p>
  <p style="color: #666; font-size: 14px;">Or copy this link: {}</p>
  <p style="color: #666; font-size: 14px;">This link will expire in 24 hours.</p>
</body>
</html>"#,
        verify_url, verify_url
    );

    let text = format!(
        "Welcome to Alelysee!\n\nPlease verify your email address by visiting this link:\n\n{}\n\nThis link will expire in 24 hours.",
        verify_url
    );

    email_service
        .send_email(to, "Verify your email address", &html, &text)
        .await
}

/// Send password reset email
pub async fn send_password_reset_email(
    email_service: &dyn EmailService,
    to: &str,
    token: &str,
) -> Result<()> {
    info!(
        "email.send_password_reset_email: to={} token_len={}",
        email_label(to),
        token.len()
    );
    let base_url =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let reset_url = format!("{}/auth/reset-password/confirm?token={}", base_url, token);

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"></head>
<body style="font-family: sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
  <h1 style="color: #333;">Reset your password</h1>
  <p>You requested to reset your password. Click the button below to set a new password:</p>
  <p style="margin: 30px 0;">
    <a href="{}" style="background-color: #007bff; color: white; padding: 12px 24px; text-decoration: none; border-radius: 4px; display: inline-block;">Reset Password</a>
  </p>
  <p style="color: #666; font-size: 14px;">Or copy this link: {}</p>
  <p style="color: #666; font-size: 14px;">This link will expire in 1 hour.</p>
  <p style="color: #666; font-size: 14px;">If you didn't request this, you can safely ignore this email.</p>
</body>
</html>"#,
        reset_url, reset_url
    );

    let text = format!(
        "You requested to reset your password.\n\nVisit this link to set a new password:\n\n{}\n\nThis link will expire in 1 hour.\n\nIf you didn't request this, you can safely ignore this email.",
        reset_url
    );

    email_service
        .send_email(to, "Reset your password", &html, &text)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token_produces_64_hex_chars() {
        let token = generate_token();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_token_is_unique() {
        let token1 = generate_token();
        let token2 = generate_token();
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_hash_token_is_deterministic() {
        let token = "abcd1234";
        let hash1 = hash_token(token);
        let hash2 = hash_token(token);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_token_produces_64_hex_chars() {
        let token = "test_token";
        let hash = hash_token(token);
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
