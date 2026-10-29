use std::time::Duration;

use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::extension::ClientId;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use super::{EmailMessage, EmailSender};
use crate::config::{EmailConfig, SmtpSecurity};
use crate::error::{AppError, ErrorCode};
use crate::shared::future::BoxFuture;

/// How long one SMTP exchange may take.
const SMTP_TIMEOUT: Duration = Duration::from_secs(20);

/// [`EmailSender`] over SMTP (SMTP2GO or any provider in production, Mailpit locally). It opens
/// a connection per email: sign-in links are few, and a dropped connection never lingers.
pub struct SmtpEmailSender {
    config: EmailConfig,
}

impl SmtpEmailSender {
    /// A sender with these settings.
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    fn transport(&self) -> Result<AsyncSmtpTransport<Tokio1Executor>, AppError> {
        let config = &self.config;
        let builder = match config.smtp_security {
            SmtpSecurity::StartTls => {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
                    .map_err(unavailable)?
            }
            SmtpSecurity::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
                .map_err(unavailable)?,
            SmtpSecurity::None => {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
            }
        };
        let mut builder = builder
            .port(config.smtp_port)
            .timeout(Some(SMTP_TIMEOUT))
            .hello_name(ClientId::Domain(hello_name(&config.from)));
        if let Some((username, password)) = &config.smtp_credentials {
            builder = builder.credentials(Credentials::new(
                username.clone(),
                password.expose().clone(),
            ));
        }
        Ok(builder.build())
    }
}

impl EmailSender for SmtpEmailSender {
    fn send<'a>(&'a self, message: &'a EmailMessage) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            let from: Mailbox = self.config.from.parse().map_err(internal)?;
            let to: Mailbox = message.to.parse().map_err(|error| {
                AppError::new(ErrorCode::InvalidRequest)
                    .with_meta("field", "email")
                    .with_source(error)
            })?;
            let email = Message::builder()
                .from(from)
                .to(to)
                .subject(&message.subject)
                .header(ContentType::TEXT_PLAIN)
                .body(message.text.clone())
                .map_err(internal)?;
            self.transport()?.send(email).await.map_err(unavailable)?;
            Ok(())
        })
    }
}

/// The name to greet the server with (EHLO): the sender's domain.
fn hello_name(from: &str) -> String {
    let address = from.rsplit_once('<').map_or(from, |(_, rest)| rest);
    let domain = address
        .rsplit_once('@')
        .map_or("localhost", |(_, domain)| domain);
    domain.trim_end_matches('>').trim().to_owned()
}

fn unavailable(error: impl std::error::Error + Send + Sync + 'static) -> AppError {
    AppError::new(ErrorCode::ServiceUnavailable)
        .with_meta("dependency", "email")
        .with_source(error)
}

fn internal(error: impl std::error::Error + Send + Sync + 'static) -> AppError {
    AppError::new(ErrorCode::Internal).with_source(error)
}
