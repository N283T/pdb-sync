//! Notification support for the watch command.
//!
//! Supports desktop notifications and email notifications, both feature-gated.

use crate::cli::args::NotifyMethod;
use crate::error::{PdbSyncError, Result};

/// Configuration for notifications
#[derive(Debug, Clone)]
pub struct NotifyConfig {
    /// Notification method
    pub method: NotifyMethod,
    /// Email address (required for email notifications)
    #[allow(dead_code)]
    pub email: Option<String>,
}

impl NotifyConfig {
    /// Create a new notification config
    pub fn new(method: NotifyMethod, email: Option<String>) -> Result<Self> {
        // Validate email is provided for email notifications
        if method == NotifyMethod::Email && email.is_none() {
            return Err(PdbSyncError::Notification(
                "Email address required for email notifications".into(),
            ));
        }

        Ok(Self { method, email })
    }
}

/// Notification sender
pub struct NotificationSender {
    config: NotifyConfig,
}

impl NotificationSender {
    /// Create a new notification sender
    pub fn new(config: NotifyConfig) -> Self {
        Self { config }
    }

    /// Send a notification about new PDB entries
    pub async fn notify(&self, pdb_ids: &[String]) -> Result<()> {
        if pdb_ids.is_empty() {
            return Ok(());
        }

        match self.config.method {
            NotifyMethod::Desktop => self.send_desktop_notification(pdb_ids).await,
            NotifyMethod::Email => self.send_email_notification(pdb_ids).await,
        }
    }

    /// Send a desktop notification
    #[cfg(feature = "desktop-notify")]
    async fn send_desktop_notification(&self, pdb_ids: &[String]) -> Result<()> {
        let count = pdb_ids.len();
        let title = format!("PDB Watch: {} new entries", count);

        let body = if count <= 5 {
            pdb_ids.join(", ")
        } else {
            format!("{}, ... and {} more", pdb_ids[..5].join(", "), count - 5)
        };

        notify_rust::Notification::new()
            .summary(&title)
            .body(&body)
            .appname("pdb-sync")
            .timeout(notify_rust::Timeout::Milliseconds(10000))
            .show()
            .map_err(|e| {
                PdbSyncError::Notification(format!("Failed to send notification: {}", e))
            })?;

        tracing::debug!("Sent desktop notification for {} entries", count);
        Ok(())
    }

    #[cfg(not(feature = "desktop-notify"))]
    async fn send_desktop_notification(&self, _pdb_ids: &[String]) -> Result<()> {
        Err(PdbSyncError::Notification(
            "Desktop notifications not enabled. Rebuild with --features desktop-notify".into(),
        ))
    }

    /// Send an email notification
    #[cfg(feature = "email-notify")]
    async fn send_email_notification(&self, pdb_ids: &[String]) -> Result<()> {
        use lettre::{
            message::header::ContentType, transport::smtp::authentication::Credentials,
            AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
        };

        let email_addr = self
            .config
            .email
            .as_ref()
            .ok_or_else(|| PdbSyncError::Notification("Email address not configured".into()))?;

        // Get SMTP configuration from environment
        let smtp_host = std::env::var("SMTP_HOST").map_err(|_| {
            PdbSyncError::Notification("SMTP_HOST environment variable not set".into())
        })?;
        let smtp_user = std::env::var("SMTP_USER").map_err(|_| {
            PdbSyncError::Notification("SMTP_USER environment variable not set".into())
        })?;
        let smtp_pass = std::env::var("SMTP_PASS").map_err(|_| {
            PdbSyncError::Notification("SMTP_PASS environment variable not set".into())
        })?;

        let count = pdb_ids.len();
        let subject = format!("PDB Watch: {} new entries downloaded", count);

        let body = format!(
            "The following {} PDB entries were downloaded:\n\n{}\n\n--\npdb-sync watch",
            count,
            pdb_ids.join("\n")
        );

        let email =
            Message::builder()
                .from(smtp_user.parse().map_err(|e| {
                    PdbSyncError::Notification(format!("Invalid from address: {}", e))
                })?)
                .to(email_addr.parse().map_err(|e| {
                    PdbSyncError::Notification(format!("Invalid to address: {}", e))
                })?)
                .subject(subject)
                .header(ContentType::TEXT_PLAIN)
                .body(body)
                .map_err(|e| PdbSyncError::Notification(format!("Failed to build email: {}", e)))?;

        let creds = Credentials::new(smtp_user.clone(), smtp_pass);

        let mailer: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host)
                .map_err(|e| {
                    PdbSyncError::Notification(format!("Failed to connect to SMTP: {}", e))
                })?
                .credentials(creds)
                .build();

        mailer
            .send(email)
            .await
            .map_err(|e| PdbSyncError::Notification(format!("Failed to send email: {}", e)))?;

        tracing::debug!(
            "Sent email notification to {} for {} entries",
            email_addr,
            count
        );
        Ok(())
    }

    #[cfg(not(feature = "email-notify"))]
    async fn send_email_notification(&self, _pdb_ids: &[String]) -> Result<()> {
        Err(PdbSyncError::Notification(
            "Email notifications not enabled. Rebuild with --features email-notify".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify_config_email_requires_address() {
        let result = NotifyConfig::new(NotifyMethod::Email, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Email address required"));
    }

    #[test]
    fn test_notify_config_email_with_address() {
        let result = NotifyConfig::new(NotifyMethod::Email, Some("test@example.com".into()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_notify_config_desktop() {
        let result = NotifyConfig::new(NotifyMethod::Desktop, None);
        assert!(result.is_ok());
    }
}
