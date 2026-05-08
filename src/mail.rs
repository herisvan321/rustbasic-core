/* ---------------------------------------------------------
 * 📑 LABEL: MAIL SERVICE (config/mail.rs)
 * Menangani pengiriman email menggunakan SMTP (Lettre).
 * --------------------------------------------------------- */

use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use crate::app::Config;

pub struct MailService;

impl MailService {
    /// Mengirim email secara asinkron
    pub async fn send_email(
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::load();

        // 1. Buat Email
        let email = Message::builder()
            .from(format!("{} <{}>", config.mail_from_name, config.mail_from_address).parse()?)
            .to(to.parse()?)
            .subject(subject)
            .header(lettre::message::header::ContentType::TEXT_HTML)
            .body(body.to_string())?;

        // 2. Konfigurasi SMTP Transport (Async)
        let creds = Credentials::new(config.mail_username, config.mail_password);

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.mail_host)?
            .port(config.mail_port)
            .credentials(creds)
            .build();

        // 3. Kirim Email (Asinkron)
        mailer.send(email).await?;

        Ok(())
    }
}
