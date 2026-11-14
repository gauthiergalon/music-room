use std::{sync::Mutex, time::Duration};

use async_trait::async_trait;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::authentication::Credentials,
};

use crate::errors::{AppError, ErrorMessage};

const DEFAULT_SENDER: &str = "Music Room <noreply@musicroom.app>";
const SMTP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct Email {
    pub subject: String,
    pub body: String,
}

impl Email {
    pub fn new(subject: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            body: body.into(),
        }
    }

    pub fn for_password_reset(token: &str) -> Self {
        Self::new(
            "Reset your password",
            format!(
                "Click on this link to reset your password...\n\nmusicroom://reset-password?token={}",
                token
            ),
        )
    }

    pub fn for_email_confirmation(token: &str) -> Self {
        Self::new(
            "Confirm your email address",
            format!(
                "Please confirm your email address by clicking on this link...\n\nmusicroom://confirm-email?token={}",
                token
            ),
        )
    }
}

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, receiver: &str, email: &Email) -> Result<(), AppError>;
}

/// How the connection to the SMTP server is secured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpSecurity {
    /// Plain text, for a local catcher such as Mailhog.
    None,
    /// Upgrade with STARTTLS (usually port 587).
    StartTls,
    /// TLS from the first byte (usually port 465).
    Tls,
}

pub struct SmtpConfig {
    pub host: String,
    pub port: Option<u16>,
    pub security: SmtpSecurity,
    pub credentials: Option<(String, String)>,
    pub sender: String,
}

impl SmtpConfig {
    /// Reads `SMTP_HOST`, `SMTP_PORT`, `SMTP_SECURITY` (`none` | `starttls` |
    /// `tls`), `SMTP_USERNAME`, `SMTP_PASSWORD` and `SMTP_FROM`. The defaults
    /// target a local Mailhog (`localhost:1025`, no TLS, no credentials).
    pub fn from_env() -> Self {
        let var = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());

        let security = match var("SMTP_SECURITY").as_deref() {
            Some("starttls") => SmtpSecurity::StartTls,
            Some("tls") => SmtpSecurity::Tls,
            _ => SmtpSecurity::None,
        };
        let port = var("SMTP_PORT").and_then(|value| value.parse().ok());

        Self {
            host: var("SMTP_HOST").unwrap_or_else(|| "localhost".to_string()),
            port: port.or((security == SmtpSecurity::None).then_some(1025)),
            security,
            credentials: var("SMTP_USERNAME").zip(var("SMTP_PASSWORD")),
            sender: var("SMTP_FROM").unwrap_or_else(|| DEFAULT_SENDER.to_string()),
        }
    }
}

pub struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    sender: Mailbox,
}

impl SmtpMailer {
    pub fn new(config: SmtpConfig) -> Result<Self, String> {
        let mut builder = match config.security {
            SmtpSecurity::None => {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
            }
            SmtpSecurity::StartTls => {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
                    .map_err(|e| format!("invalid SMTP configuration: {e}"))?
            }
            SmtpSecurity::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
                .map_err(|e| format!("invalid SMTP configuration: {e}"))?,
        };

        if let Some(port) = config.port {
            builder = builder.port(port);
        }
        if let Some((username, password)) = config.credentials {
            builder = builder.credentials(Credentials::new(username, password));
        }

        Ok(Self {
            transport: builder.timeout(Some(SMTP_TIMEOUT)).build(),
            sender: config
                .sender
                .parse()
                .map_err(|e| format!("invalid SMTP_FROM address: {e}"))?,
        })
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, receiver: &str, email: &Email) -> Result<(), AppError> {
        let message = Message::builder()
            .from(self.sender.clone())
            .to(receiver
                .parse()
                .map_err(|_| AppError::Validation(vec![ErrorMessage::EmailInvalidFormat]))?)
            .subject(&email.subject)
            .header(ContentType::TEXT_PLAIN)
            .body(email.body.clone())
            .map_err(|e| AppError::InternalError(format!("Failed to build email: {e}")))?;

        self.transport.send(message).await.map_err(|e| {
            tracing::error!("Failed to send email: {e}");
            AppError::BadGateway(ErrorMessage::UpstreamUnavailable)
        })?;

        Ok(())
    }
}

/// Keeps sent emails in memory instead of delivering them. Used by the tests,
/// which read the tokens back from the message bodies.
#[derive(Default)]
pub struct MemoryMailer {
    sent: Mutex<Vec<(String, Email)>>,
}

impl MemoryMailer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every email sent so far, oldest first, as `(receiver, email)`.
    pub fn sent(&self) -> Vec<(String, Email)> {
        self.sent.lock().expect("mailer lock poisoned").clone()
    }
}

#[async_trait]
impl Mailer for MemoryMailer {
    async fn send(&self, receiver: &str, email: &Email) -> Result<(), AppError> {
        self.sent
            .lock()
            .expect("mailer lock poisoned")
            .push((receiver.to_string(), email.clone()));
        Ok(())
    }
}
