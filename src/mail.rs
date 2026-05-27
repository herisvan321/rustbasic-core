/* ---------------------------------------------------------
 * 📧 LABEL: MAIL SERVICE (src/mail.rs)
 * Menangani pengiriman email dengan SMTP menggunakan Lettre.
 * Menyediakan Mailer fluent builder API yang kaya fitur.
 * --------------------------------------------------------- */

use std::error::Error;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use crate::app::Config;

/// Representasi attachment / lampiran email
#[derive(Debug, Clone)]
pub struct MailAttachment {
    pub name: String,
    pub body: Vec<u8>,
    pub content_type: String,
}

/// Fluent Builder untuk menyusun dan mengirim Email
#[derive(Debug, Clone)]
pub struct Mailer {
    to: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    subject: Option<String>,
    html_body: Option<String>,
    text_body: Option<String>,
    from_name: Option<String>,
    from_address: Option<String>,
    attachments: Vec<MailAttachment>,
}

impl Default for Mailer {
    fn default() -> Self {
        Self::new()
    }
}

impl Mailer {
    /// Membuat instance baru Mailer
    pub fn new() -> Self {
        Self {
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: None,
            html_body: None,
            text_body: None,
            from_name: None,
            from_address: None,
            attachments: Vec::new(),
        }
    }

    /// Menambahkan penerima utama (To)
    pub fn to(mut self, to: impl Into<String>) -> Self {
        self.to.push(to.into());
        self
    }

    /// Menambahkan CC (Carbon Copy)
    pub fn cc(mut self, cc: impl Into<String>) -> Self {
        self.cc.push(cc.into());
        self
    }

    /// Menambahkan BCC (Blind Carbon Copy)
    pub fn bcc(mut self, bcc: impl Into<String>) -> Self {
        self.bcc.push(bcc.into());
        self
    }

    /// Mengatur subjek email
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Mengatur konten HTML body email
    pub fn html(mut self, body: impl Into<String>) -> Self {
        self.html_body = Some(body.into());
        self
    }

    /// Mengatur konten Text biasa body email
    pub fn text(mut self, body: impl Into<String>) -> Self {
        self.text_body = Some(body.into());
        self
    }

    /// Mengatur kustom pengirim (From) untuk email ini
    pub fn from(mut self, name: impl Into<String>, address: impl Into<String>) -> Self {
        self.from_name = Some(name.into());
        self.from_address = Some(address.into());
        self
    }

    /// Melampirkan file berkas ke email
    pub fn attach(mut self, name: impl Into<String>, body: Vec<u8>, content_type: impl Into<String>) -> Self {
        self.attachments.push(MailAttachment {
            name: name.into(),
            body,
            content_type: content_type.into(),
        });
        self
    }

    /// Mengirim email secara asinkron menggunakan SMTP relay dari Config
    pub async fn send(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let config = Config::load();

        let from_name = self.from_name.unwrap_or(config.mail_from_name);
        let from_address = self.from_address.unwrap_or(config.mail_from_address);

        let mut builder = Message::builder()
            .from(format!("{} <{}>", from_name, from_address).parse()?);

        if self.to.is_empty() {
            return Err("At least one recipient ('to') must be specified".into());
        }

        for recipient in self.to {
            builder = builder.to(recipient.parse()?);
        }

        for cc_rec in self.cc {
            builder = builder.cc(cc_rec.parse()?);
        }

        for bcc_rec in self.bcc {
            builder = builder.bcc(bcc_rec.parse()?);
        }

        if let Some(sub) = self.subject {
            builder = builder.subject(sub);
        }

        // Tentukan body (Text / HTML / Alternative)
        let email = match (self.html_body, self.text_body) {
            (Some(html), Some(text)) => {
                let alt = lettre::message::MultiPart::alternative()
                    .singlepart(lettre::message::SinglePart::plain(text))
                    .singlepart(lettre::message::SinglePart::html(html));
                if !self.attachments.is_empty() {
                    let mut mixed = lettre::message::MultiPart::mixed().multipart(alt);
                    for att in self.attachments {
                        let mime = att.content_type.parse::<lettre::message::header::ContentType>()?;
                        let single_part = lettre::message::SinglePart::builder()
                            .header(mime)
                            .header(lettre::message::header::ContentDisposition::attachment(&att.name))
                            .body(att.body);
                        mixed = mixed.singlepart(single_part);
                    }
                    builder.multipart(mixed)?
                } else {
                    builder.multipart(alt)?
                }
            }
            (Some(html), None) => {
                if !self.attachments.is_empty() {
                    let alt = lettre::message::MultiPart::alternative()
                        .singlepart(lettre::message::SinglePart::html(html));
                    let mut mixed = lettre::message::MultiPart::mixed().multipart(alt);
                    for att in self.attachments {
                        let mime = att.content_type.parse::<lettre::message::header::ContentType>()?;
                        let single_part = lettre::message::SinglePart::builder()
                            .header(mime)
                            .header(lettre::message::header::ContentDisposition::attachment(&att.name))
                            .body(att.body);
                        mixed = mixed.singlepart(single_part);
                    }
                    builder.multipart(mixed)?
                } else {
                    builder
                        .header(lettre::message::header::ContentType::TEXT_HTML)
                        .body(html)?
                }
            }
            (None, Some(text)) => {
                if !self.attachments.is_empty() {
                    let alt = lettre::message::MultiPart::alternative()
                        .singlepart(lettre::message::SinglePart::plain(text));
                    let mut mixed = lettre::message::MultiPart::mixed().multipart(alt);
                    for att in self.attachments {
                        let mime = att.content_type.parse::<lettre::message::header::ContentType>()?;
                        let single_part = lettre::message::SinglePart::builder()
                            .header(mime)
                            .header(lettre::message::header::ContentDisposition::attachment(&att.name))
                            .body(att.body);
                        mixed = mixed.singlepart(single_part);
                    }
                    builder.multipart(mixed)?
                } else {
                    builder
                        .header(lettre::message::header::ContentType::TEXT_PLAIN)
                        .body(text)?
                }
            }
            (None, None) => {
                return Err("Email body (HTML or Text) is required".into());
            }
        };

        // Konfigurasi SMTP Transport (Async)
        let creds = Credentials::new(config.mail_username, config.mail_password);

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.mail_host)?
            .port(config.mail_port)
            .credentials(creds)
            .build();

        mailer.send(email).await?;

        Ok(())
    }
}

/// Helper kompatibilitas lama
pub struct MailService;

impl MailService {
    /// Mengirim email secara asinkron menggunakan SMTP
    pub async fn send_email(
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Mailer::new()
            .to(to)
            .subject(subject)
            .html(body)
            .send()
            .await
    }
}
