//! `System.Net.Mail` as the hub uses it: `MailMessage`, `MailAddress` and `SendMail`.
//!
//! The hub builds a [`MailMessage`] on the hub thread exactly as the C# does, then hands it to a
//! [`Mailer`]. The real mailer sends it over SMTP on a separate task (package T, lettre), so a
//! slow mail server never blocks the hub; C# awaited the send, but its only effect was a console
//! line on failure (GameHub.cs:147-150). The equivalence harness records the messages instead.

/// `MailPriority`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailPriority {
    Normal,
    Low,
    High,
}

/// `new Attachment(stream, name)`: the hub only attaches text (the savegame).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    pub name: String,
    pub content: String,
}

/// `MailAddress`. Constructing one validates the address and throws `FormatException` for an
/// invalid one, *before* `SendMail` is reached and outside its try/catch. Several hub methods
/// depend on that (see the plan, "Hazards"), so [`MailAddress::parse`] accepts and rejects
/// exactly what .NET does. It is a line-by-line port of .NET 10's `MailAddressParser` and its
/// readers (`System.Net.Mail`, runtime tag v10.0.12), checked against .NET with
/// `tests/mail_address.rs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailAddress {
    /// `DisplayName`.
    pub display_name: String,
    /// `User`.
    pub user: String,
    /// `Host`.
    pub host: String,
}

/// The exception `new MailAddress(address)` throws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailAddressError {
    /// `FormatException`, or `ArgumentException` for an empty address.
    pub type_name: &'static str,
    pub message: String,
}

impl std::fmt::Display for MailAddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.type_name, self.message)
    }
}

impl From<MailAddressError> for crate::hub::HubError {
    fn from(e: MailAddressError) -> Self {
        crate::hub::HubError::Exception { type_name: e.type_name, message: e.message }
    }
}

impl MailAddress {
    /// `new MailAddress(address)` (MailAddress.cs `TryParse` with `throwExceptionIfFail: true`).
    pub fn parse(address: &str) -> Result<MailAddress, MailAddressError> {
        // ArgumentException.ThrowIfNullOrEmpty(address)
        if address.is_empty() {
            return Err(MailAddressError {
                type_name: "ArgumentException",
                message: "The value cannot be an empty string. (Parameter 'address')".to_string(),
            });
        }
        let data: Vec<u16> = address.encode_utf16().collect();
        let info = parser::try_parse_address(&data)?;
        Ok(MailAddress { display_name: info.display_name, user: info.user, host: info.host })
    }

    /// `Address`: `user@host`.
    pub fn address(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }
}

/// `MailAddress.ToString()`.
impl std::fmt::Display for MailAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.display_name.is_empty() {
            write!(f, "{}", self.address())
        } else {
            write!(f, "\"{}\" <{}>", self.display_name.replace('"', "\\\""), self.address())
        }
    }
}

/// `MailAddressParser`, `MailBnfHelper` and the readers, for a single address
/// (`expectMultipleAddresses` is false on this path, but kept as a parameter to mirror the C#).
/// The C# works on UTF-16 code units with indices that run backwards to -1; so does this.
mod parser {
    use super::MailAddressError;

    const QUOTE: u16 = b'"' as u16;
    const SPACE: u16 = b' ' as u16;
    const TAB: u16 = b'\t' as u16;
    const CR: u16 = b'\r' as u16;
    const LF: u16 = b'\n' as u16;
    const START_COMMENT: u16 = b'(' as u16;
    const END_COMMENT: u16 = b')' as u16;
    const BACKSLASH: u16 = b'\\' as u16;
    const AT: u16 = b'@' as u16;
    const END_ANGLE_BRACKET: u16 = b'>' as u16;
    const START_ANGLE_BRACKET: u16 = b'<' as u16;
    const START_SQUARE_BRACKET: u16 = b'[' as u16;
    const END_SQUARE_BRACKET: u16 = b']' as u16;
    const COMMA: u16 = b',' as u16;
    const DOT: u16 = b'.' as u16;

    pub struct ParseAddressInfo {
        pub display_name: String,
        pub user: String,
        pub host: String,
    }

    type R<T> = Result<T, MailAddressError>;

    /// `SR.MailAddressInvalidFormat`.
    fn invalid_format() -> MailAddressError {
        MailAddressError { type_name: "FormatException", message: "The specified string is not in the form required for an e-mail address.".to_string() }
    }

    /// `SR.Format(SR.MailHeaderFieldInvalidCharacter, x)`. A lone surrogate becomes U+FFFD.
    fn invalid_character(x: &[u16]) -> MailAddressError {
        MailAddressError {
            type_name: "FormatException",
            message: format!("An invalid character was found in the mail header: '{}'.", String::from_utf16_lossy(x)),
        }
    }

    fn at(data: &[u16], index: i32) -> u16 {
        data[index as usize]
    }

    /// `Ascii.IsValid(char)`.
    fn is_ascii(c: u16) -> bool {
        c <= 0x7F
    }

    /// `MailBnfHelper.Atext`.
    fn atext(c: u16) -> bool {
        matches!(c as u8, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z')
            || b"!#$%&'*+-/=?^_`{|}~".contains(&(c as u8))
    }

    /// `MailBnfHelper.Qtext`.
    fn qtext(c: u16) -> bool {
        matches!(c, 1..=9 | 11 | 12 | 14..=33 | 35..=91 | 93..=127)
    }

    /// `MailBnfHelper.Dtext`.
    fn dtext(c: u16) -> bool {
        matches!(c, 1..=8 | 11 | 12 | 14..=31 | 33..=90 | 94..=127)
    }

    /// `MailBnfHelper.Ctext`.
    fn ctext(c: u16) -> bool {
        matches!(c, 1..=8 | 11 | 12 | 14..=31 | 33..=39 | 42..=91 | 93..=127)
    }

    /// `MailBnfHelper.IsAllowedWhiteSpace`.
    fn is_allowed_white_space(c: u16) -> bool {
        c == TAB || c == SPACE || c == CR || c == LF
    }

    /// `data.Substring(start, length)`.
    fn substring(data: &[u16], start: i32, length: i32) -> &[u16] {
        &data[start as usize..(start + length) as usize]
    }

    /// `TryParseAddress(string data, out ParseAddressInfo, bool)` (MailAddressParser.cs:26)
    /// and the private overload it calls (:65) with `expectMultipleAddresses: false`.
    pub fn try_parse_address(data: &[u16]) -> R<ParseAddressInfo> {
        let expect_multiple_addresses = false;
        let mut index = data.len() as i32 - 1;

        // MailBnfHelper.HasCROrLF
        if data.iter().any(|&c| c == CR || c == LF) {
            return Err(invalid_format());
        }

        index = try_read_cfws_and_throw_if_incomplete(data, index)?;

        let mut expect_angle_bracket = false;
        if at(data, index) == END_ANGLE_BRACKET {
            expect_angle_bracket = true;
            index -= 1;
        }

        let domain;
        (domain, index) = try_parse_domain(data, index)?;

        if at(data, index) != AT {
            return Err(invalid_format());
        }

        index -= 1;

        let local_part;
        (local_part, index) = try_parse_local_part(data, index, expect_angle_bracket, expect_multiple_addresses)?;

        if expect_angle_bracket {
            if index >= 0 && at(data, index) == START_ANGLE_BRACKET {
                index -= 1;
                index = try_read_fws_reverse(data, index)?;
            } else {
                let c = if index >= 0 { at(data, index) } else { END_ANGLE_BRACKET };
                return Err(invalid_character(&[c]));
            }
        }

        let display_name = if index >= 0 && !(expect_multiple_addresses && at(data, index) == COMMA) {
            let (display_name, _) = try_parse_display_name(data, index, expect_multiple_addresses)?;
            display_name
        } else {
            String::new()
        };

        Ok(ParseAddressInfo { display_name, user: local_part, host: domain })
    }

    /// `TryReadCfwsAndThrowIfIncomplete` (MailAddressParser.cs:183).
    fn try_read_cfws_and_throw_if_incomplete(data: &[u16], index: i32) -> R<i32> {
        let index = try_read_cfws_reverse(data, index)?;
        if index < 0 {
            return Err(invalid_format());
        }
        Ok(index)
    }

    /// `TryParseDomain` (MailAddressParser.cs:223). Returns the domain and the new index.
    fn try_parse_domain(data: &[u16], index: i32) -> R<(String, i32)> {
        let mut index = try_read_cfws_and_throw_if_incomplete(data, index)?;

        let starting_index = index;

        if at(data, index) == END_SQUARE_BRACKET {
            index = domain_literal_try_read_reverse(data, index)?;
        } else {
            index = dot_atom_try_read_reverse(data, index)?;
        }

        let domain = substring(data, index + 1, starting_index - index);

        index = try_read_cfws_and_throw_if_incomplete(data, index)?;

        Ok((try_normalize_or_throw(domain)?, index))
    }

    /// `TryParseLocalPart` (MailAddressParser.cs:285).
    fn try_parse_local_part(data: &[u16], index: i32, expect_angle_bracket: bool, expect_multiple_addresses: bool) -> R<(String, i32)> {
        let mut index = try_read_cfws_and_throw_if_incomplete(data, index)?;

        let starting_index = index;

        if at(data, index) == QUOTE {
            index = try_read_reverse_quoted(data, index, true)?;
        } else {
            index = dot_atom_try_read_reverse(data, index)?;

            if index >= 0 {
                let c = at(data, index);
                if !(is_allowed_white_space(c)
                    || c == END_COMMENT
                    || (expect_angle_bracket && c == START_ANGLE_BRACKET)
                    || (expect_multiple_addresses && c == COMMA)
                    || c == QUOTE)
                {
                    return Err(invalid_character(&[c]));
                }
            }
        }

        let local_part = substring(data, index + 1, starting_index - index);

        index = try_read_cfws_reverse(data, index)?;

        Ok((try_normalize_or_throw(local_part)?, index))
    }

    /// `TryParseDisplayName` (MailAddressParser.cs:369).
    fn try_parse_display_name(data: &[u16], index: i32, expect_multiple_addresses: bool) -> R<(String, i32)> {
        let mut index = index;
        let first_non_comment_index = try_read_cfws_reverse(data, index)?;

        let display_name: String;
        if first_non_comment_index >= 0 && at(data, first_non_comment_index) == QUOTE {
            index = try_read_reverse_quoted(data, first_non_comment_index, true)?;

            let left_index = index + 2;
            let name = substring(data, left_index, first_non_comment_index - left_index);

            index = try_read_cfws_reverse(data, index)?;

            if index >= 0 && !(expect_multiple_addresses && at(data, index) == COMMA) {
                return Err(invalid_character(&[at(data, index)]));
            }
            display_name = try_normalize_or_throw(name)?;
        } else {
            let starting_index = index;

            index = try_read_reverse_un_quoted(data, index, true, expect_multiple_addresses)?;

            // data.AsSpan(index + 1, startingIndex - index).Trim(): char.IsWhiteSpace per UTF-16
            // unit. Every .NET white-space char is in the BMP, and the set equals Rust's
            // White_Space (checked over all of UTF-16 in tests/auth_dotnet.rs).
            let span = substring(data, index + 1, starting_index - index);
            let is_ws = |c: &u16| char::from_u32(u32::from(*c)).is_some_and(char::is_whitespace);
            let start = span.iter().position(|c| !is_ws(c)).unwrap_or(span.len());
            let end = span.iter().rposition(|c| !is_ws(c)).map_or(start, |e| e + 1);
            display_name = try_normalize_or_throw(&span[start..end])?;
        }

        Ok((display_name, index))
    }

    /// `TryNormalizeOrThrow` (MailAddressParser.cs:451): `Normalize(FormC)`, which throws
    /// `ArgumentException` (rethrown as `FormatException`) on invalid UTF-16.
    fn try_normalize_or_throw(input: &[u16]) -> R<String> {
        use unicode_normalization::UnicodeNormalization;
        match String::from_utf16(input) {
            Ok(s) => Ok(s.nfc().collect()),
            Err(_) => Err(invalid_format()),
        }
    }

    /// `DotAtomReader.TryReadReverse`.
    fn dot_atom_try_read_reverse(data: &[u16], index: i32) -> R<i32> {
        let start_index = index;
        let mut index = index;

        while 0 <= index {
            let c = at(data, index);
            if is_ascii(c) && ((c != DOT && !atext(c)) || (c == DOT && index > 0 && at(data, index - 1) == DOT)) {
                break;
            }
            index -= 1;
        }

        if start_index == index {
            Err(invalid_character(&[at(data, index)]))
        } else if index > 0 && at(data, index) == DOT && at(data, index - 1) == DOT {
            Err(invalid_character(&[DOT, DOT]))
        } else if at(data, index + 1) == DOT {
            Err(invalid_character(&[DOT]))
        } else {
            Ok(index)
        }
    }

    /// `QuotedPairReader.TryCountQuotedChars`.
    fn try_count_quoted_chars(data: &[u16], index: i32, permit_unicode_escaping: bool) -> R<i32> {
        if index <= 0 || at(data, index - 1) != BACKSLASH {
            return Ok(0);
        }

        let backslash_count = count_backslashes(data, index - 1);

        if backslash_count % 2 == 0 {
            Ok(0)
        } else {
            if !permit_unicode_escaping && !is_ascii(at(data, index)) {
                return Err(invalid_character(&[at(data, index)]));
            }
            Ok(backslash_count + 1)
        }
    }

    /// `QuotedPairReader.CountBackslashes`.
    fn count_backslashes(data: &[u16], index: i32) -> i32 {
        let mut index = index;
        let mut backslash_count = 0;
        loop {
            backslash_count += 1;
            index -= 1;
            if !(index >= 0 && at(data, index) == BACKSLASH) {
                break;
            }
        }
        backslash_count
    }

    /// `DomainLiteralReader.TryReadReverse`.
    fn domain_literal_try_read_reverse(data: &[u16], index: i32) -> R<i32> {
        let mut index = index - 1;

        loop {
            index = try_read_fws_reverse(data, index)?;

            if index < 0 {
                break;
            }

            let quoted_char_count = try_count_quoted_chars(data, index, false)?;

            if quoted_char_count > 0 {
                index -= quoted_char_count;
            } else if at(data, index) == START_SQUARE_BRACKET {
                return Ok(index - 1);
            } else if !is_ascii(at(data, index)) || !dtext(at(data, index)) {
                return Err(invalid_character(&[at(data, index)]));
            } else {
                index -= 1;
            }

            if index < 0 {
                break;
            }
        }

        Err(invalid_character(&[END_SQUARE_BRACKET]))
    }

    /// `QuotedStringFormatReader.TryReadReverseQuoted`.
    fn try_read_reverse_quoted(data: &[u16], index: i32, permit_unicode: bool) -> R<i32> {
        let mut index = index - 1;

        loop {
            index = try_read_fws_reverse(data, index)?;

            if index < 0 {
                break;
            }

            let quoted_char_count = try_count_quoted_chars(data, index, permit_unicode)?;

            if quoted_char_count > 0 {
                index -= quoted_char_count;
            } else if at(data, index) == QUOTE {
                return Ok(index - 1);
            } else if !is_valid_qtext(permit_unicode, at(data, index)) {
                return Err(invalid_character(&[at(data, index)]));
            } else {
                index -= 1;
            }

            if index < 0 {
                break;
            }
        }

        Err(invalid_character(&[QUOTE]))
    }

    /// `QuotedStringFormatReader.TryReadReverseUnQuoted`.
    fn try_read_reverse_un_quoted(data: &[u16], index: i32, permit_unicode: bool, expect_comma_delimiter: bool) -> R<i32> {
        let mut index = index;

        loop {
            index = try_read_fws_reverse(data, index)?;

            if index < 0 {
                break;
            }

            let quoted_char_count = try_count_quoted_chars(data, index, permit_unicode)?;

            if quoted_char_count > 0 {
                index -= quoted_char_count;
            } else if expect_comma_delimiter && at(data, index) == COMMA {
                break;
            } else if !is_valid_qtext(permit_unicode, at(data, index)) {
                return Err(invalid_character(&[at(data, index)]));
            } else {
                index -= 1;
            }

            if index < 0 {
                break;
            }
        }

        Ok(index)
    }

    /// `QuotedStringFormatReader.IsValidQtext`.
    fn is_valid_qtext(allow_unicode: bool, ch: u16) -> bool {
        if !is_ascii(ch) {
            allow_unicode
        } else {
            qtext(ch)
        }
    }

    /// `WhitespaceReader.TryReadFwsReverse`.
    fn try_read_fws_reverse(data: &[u16], index: i32) -> R<i32> {
        let mut index = index;
        let mut expect_cr = false;

        while index >= 0 {
            let c = at(data, index);
            if c == CR && expect_cr {
                expect_cr = false;
            } else if c == CR || expect_cr {
                return Err(invalid_format());
            } else if c == LF {
                expect_cr = true;
            } else if c == SPACE || c == TAB {
            } else {
                break;
            }
            index -= 1;
        }

        if expect_cr {
            return Err(invalid_format());
        }

        Ok(index)
    }

    /// `WhitespaceReader.TryReadCfwsReverse`.
    fn try_read_cfws_reverse(data: &[u16], index: i32) -> R<i32> {
        let mut comment_depth = 0;

        let mut index = try_read_fws_reverse(data, index)?;

        while index >= 0 {
            let quoted_char_count = try_count_quoted_chars(data, index, true)?;
            let c = at(data, index);

            if comment_depth > 0 && quoted_char_count > 0 {
                index -= quoted_char_count;
            } else if c == END_COMMENT {
                comment_depth += 1;
                index -= 1;
            } else if c == START_COMMENT {
                comment_depth -= 1;
                if comment_depth < 0 {
                    return Err(invalid_character(&[START_COMMENT]));
                }
                index -= 1;
            } else if comment_depth > 0 && (!is_ascii(c) || ctext(c)) {
                index -= 1;
            } else if comment_depth > 0 {
                return Err(invalid_character(&[c]));
            } else {
                break;
            }

            index = try_read_fws_reverse(data, index)?;
        }

        if comment_depth > 0 {
            return Err(invalid_character(&[END_COMMENT]));
        }

        Ok(index)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailMessage {
    pub from: MailAddress,
    pub to: Vec<MailAddress>,
    pub subject: String,
    pub body: String,
    pub is_body_html: bool,
    pub priority: MailPriority,
    pub attachments: Vec<Attachment>,
}

/// SMTP settings of `SendMail` (GameHub.cs:128): host `smtp.strato.com`, `EnableSsl = true`
/// (STARTTLS on `SmtpClient`'s default port 25), credentials from the configuration.
#[derive(Clone, Debug)]
pub struct SmtpSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

pub trait Mailer {
    /// Sends without waiting. Failures are logged, as `SendMail`'s catch does.
    fn send(&self, settings: &SmtpSettings, mail: MailMessage);
}

/// Keeps every message. For tests and the harness.
#[derive(Default)]
pub struct RecordingMailer {
    pub sent: std::cell::RefCell<Vec<MailMessage>>,
}

impl Mailer for RecordingMailer {
    fn send(&self, _settings: &SmtpSettings, mail: MailMessage) {
        self.sent.borrow_mut().push(mail);
    }
}

/// The production mailer: `SmtpClient` with `EnableSsl` (GameHub.cs:139-147), as lettre over
/// STARTTLS with the configured credentials. The message is built on the hub thread and sent on
/// a separate task, so a slow mail server never holds up the hub. A failure only prints a line,
/// as `SendMail`'s catch does.
pub struct SmtpMailer;

impl Mailer for SmtpMailer {
    fn send(&self, settings: &SmtpSettings, mail: MailMessage) {
        use lettre::AsyncTransport;
        let message = match smtp::build(&mail) {
            Ok(m) => m,
            Err(e) => {
                println!("Error sending mail: {e}");
                return;
            }
        };
        let transport = match lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::starttls_relay(&settings.host) {
            Ok(t) => t
                .port(settings.port)
                .credentials(lettre::transport::smtp::authentication::Credentials::new(settings.username.clone(), settings.password.clone()))
                .build(),
            Err(e) => {
                println!("Error sending mail: {e}");
                return;
            }
        };
        tokio::spawn(async move {
            if let Err(e) = transport.send(message).await {
                println!("Error sending mail: {e}");
            }
        });
    }
}

/// `MailMessage` to a MIME message, with the headers .NET writes for the parts the hub sets.
pub mod smtp {
    use lettre::message::header::{ContentType, HeaderName, HeaderValue};
    use lettre::message::{Attachment as MimeAttachment, Mailbox, MultiPart, SinglePart};
    use lettre::Address;

    use super::{MailAddress, MailMessage, MailPriority};

    fn mailbox(a: &MailAddress) -> Result<Mailbox, String> {
        let address = Address::new(&a.user, &a.host).map_err(|e| format!("{}: {e}", a.address()))?;
        Ok(Mailbox::new((!a.display_name.is_empty()).then(|| a.display_name.clone()), address))
    }

    pub fn build(mail: &MailMessage) -> Result<lettre::Message, String> {
        let mut builder = lettre::Message::builder().from(mailbox(&mail.from)?).subject(mail.subject.clone());
        for to in &mail.to {
            builder = builder.to(mailbox(to)?);
        }
        let body_type = if mail.is_body_html { ContentType::TEXT_HTML } else { ContentType::TEXT_PLAIN };
        let body = SinglePart::builder().header(body_type).body(mail.body.clone());
        let mut message = if mail.attachments.is_empty() {
            builder.singlepart(body)
        } else {
            let mut parts = MultiPart::mixed().singlepart(body);
            for a in &mail.attachments {
                // `new Attachment(stream, name)`: application/octet-stream.
                let ct = ContentType::parse("application/octet-stream").expect("a valid content type");
                parts = parts.singlepart(MimeAttachment::new(a.name.clone()).body(a.content.clone().into_bytes(), ct));
            }
            builder.multipart(parts)
        }
        .map_err(|e| e.to_string())?;
        // .NET's MailMessage writes these three headers for a priority other than Normal.
        let priority = match mail.priority {
            MailPriority::Normal => None,
            MailPriority::Low => Some(("5", "non-urgent", "low")),
            MailPriority::High => Some(("1", "urgent", "high")),
        };
        if let Some((x_priority, priority, importance)) = priority {
            for (name, value) in [("X-Priority", x_priority), ("Priority", priority), ("Importance", importance)] {
                message.headers_mut().insert_raw(HeaderValue::new(HeaderName::new_from_ascii_str(name), value.to_string()));
            }
        }
        Ok(message)
    }
}
