//! Email sending via Resend API.
//!
//! Simple async client for Resend's REST API. No SDK dependency --
//! just a single POST to `https://api.resend.com/emails`.

use async_trait::async_trait;
use serde::Serialize;

/// Trait for sending emails, allowing test doubles.
#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send(&self, email: &Email) -> Result<(), EmailError>;
}

#[derive(Debug, Clone)]
pub struct Email {
    pub to: String,
    pub subject: String,
    pub html: String,
}

#[derive(Debug, thiserror::Error)]
pub enum EmailError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("Not configured")]
    NotConfigured,
}

/// Resend email sender.
#[derive(Clone)]
pub struct ResendSender {
    api_key: String,
    from: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct ResendRequest<'a> {
    from: &'a str,
    to: &'a [&'a str],
    subject: &'a str,
    html: &'a str,
}

impl ResendSender {
    pub fn new(api_key: impl Into<String>, from: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            from: from.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Result<Self, EmailError> {
        let api_key = std::env::var("RESEND_API_KEY")
            .map_err(|_| EmailError::NotConfigured)?;
        let from = std::env::var("RESEND_FROM")
            .unwrap_or_else(|_| "Traceway <noreply@traceway.dev>".to_string());
        Ok(Self::new(api_key, from))
    }
}

#[async_trait]
impl EmailSender for ResendSender {
    async fn send(&self, email: &Email) -> Result<(), EmailError> {
        let to_str = email.to.as_str();
        let body = ResendRequest {
            from: &self.from,
            to: &[to_str],
            subject: &email.subject,
            html: &email.html,
        };

        let resp = self
            .client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| EmailError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(EmailError::Api { status, message });
        }

        Ok(())
    }
}

/// No-op sender for local mode / tests.
pub struct NoopEmailSender;

#[async_trait]
impl EmailSender for NoopEmailSender {
    async fn send(&self, email: &Email) -> Result<(), EmailError> {
        tracing::debug!(to = %email.to, subject = %email.subject, "Email suppressed (noop sender)");
        Ok(())
    }
}
