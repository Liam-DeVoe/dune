//! Outgoing mail. The server builds [`Mail`]s and hands them to a [`Mailer`]; the SMTP mailer
//! sends each on its own task, so a slow mail server never holds up the game thread.

use std::cell::RefCell;

use lettre::message::header::{ContentType, HeaderName, HeaderValue};
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};

use super::settings::Smtp;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mail {
    pub from: String,
    pub to: String,
    pub subject: String,
    /// HTML.
    pub body: String,
    /// File name and text content.
    pub attachments: Vec<(String, String)>,
    pub low_priority: bool,
}

pub trait Mailer {
    /// Sends without waiting. Failures are logged.
    fn send(&self, mail: Mail);
}

/// Whether `address` is a mail address the server can send to.
pub fn is_valid_address(address: &str) -> bool {
    address.trim().parse::<Mailbox>().is_ok()
}

/// Logs mails instead of sending them: no SMTP login is configured.
pub struct LogMailer;

impl Mailer for LogMailer {
    fn send(&self, mail: Mail) {
        println!("mail not sent (no SMTP login set): {} to {}", mail.subject, mail.to);
    }
}

/// Keeps every mail, for tests.
#[derive(Default)]
pub struct RecordingMailer {
    pub sent: RefCell<Vec<Mail>>,
}

impl Mailer for RecordingMailer {
    fn send(&self, mail: Mail) {
        self.sent.borrow_mut().push(mail);
    }
}

/// SMTP with STARTTLS on port 25.
pub struct SmtpMailer {
    pub smtp: Smtp,
}

impl Mailer for SmtpMailer {
    fn send(&self, mail: Mail) {
        use lettre::AsyncTransport;
        let message = match build(&mail) {
            Ok(m) => m,
            Err(e) => {
                println!("mail '{}' to {}: {e}", mail.subject, mail.to);
                return;
            }
        };
        let transport = match lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::starttls_relay(&self.smtp.host) {
            Ok(t) => t
                .port(25)
                .credentials(lettre::transport::smtp::authentication::Credentials::new(self.smtp.user.clone(), self.smtp.password.clone()))
                .build(),
            Err(e) => {
                println!("mail server {}: {e}", self.smtp.host);
                return;
            }
        };
        tokio::spawn(async move {
            if let Err(e) = transport.send(message).await {
                println!("mail '{}' to {}: {e}", mail.subject, mail.to);
            }
        });
    }
}

/// The MIME message for a mail.
pub fn build(mail: &Mail) -> Result<lettre::Message, String> {
    let mailbox = |a: &str| a.trim().parse::<Mailbox>().map_err(|e| format!("{a}: {e}"));
    let builder = lettre::Message::builder().from(mailbox(&mail.from)?).to(mailbox(&mail.to)?).subject(mail.subject.clone());
    let body = SinglePart::builder().header(ContentType::TEXT_HTML).body(mail.body.clone());
    let mut message = if mail.attachments.is_empty() {
        builder.singlepart(body)
    } else {
        let mut parts = MultiPart::mixed().singlepart(body);
        for (name, content) in &mail.attachments {
            let ct = ContentType::parse("application/octet-stream").expect("a valid content type");
            parts = parts.singlepart(Attachment::new(name.clone()).body(content.clone().into_bytes(), ct));
        }
        builder.multipart(parts)
    }
    .map_err(|e| e.to_string())?;
    if mail.low_priority {
        for (name, value) in [("X-Priority", "5"), ("Priority", "non-urgent"), ("Importance", "low")] {
            message.headers_mut().insert_raw(HeaderValue::new(HeaderName::new_from_ascii_str(name), value.to_string()));
        }
    }
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert!(is_valid_address("alice@example.com"));
        assert!(is_valid_address(" Alice <alice@example.com> "));
        assert!(!is_valid_address("alice"));
        assert!(!is_valid_address(""));
    }

    #[test]
    fn builds_mime() {
        let mail = Mail {
            from: "noreply@treachery.online".into(),
            to: "alice@example.com".into(),
            subject: "Update".into(),
            body: "<p>hi</p>".into(),
            attachments: vec![("savegame.json".into(), "{}".into())],
            low_priority: true,
        };
        let text = String::from_utf8(build(&mail).unwrap().formatted()).unwrap();
        assert!(text.contains("Subject: Update") && text.contains("X-Priority: 5") && text.contains("savegame.json"), "{text}");
    }
}
