use crate::error::{GmailError, MissingParam};
use imap::{self, types::Mailbox};
use lettre::{
    message::MessageBuilder,
    transport::smtp::authentication::{self as auth},
    SmtpTransport, Transport,
};
use native_tls::{TlsConnector, TlsStream};
use std::net::TcpStream;

type Session = imap::Session<TlsStream<TcpStream>>;

pub struct GmailInbox {
    session: Option<(Session, Mailbox)>,
    credentials: Credentials,
}

struct Credentials {
    username: String,
    pass: String,
}

#[derive(Debug)]
pub struct EmailHeader {
    pub from: String,
    pub date: String,
    pub subject: String,
    pub uid: Uid,
}

pub type Offset = u32;
pub use imap::types::Uid;

const PAGE_SIZE: u8 = 20;

impl GmailInbox {
    const HEADER_QUERY: &'static str = "(ENVELOPE UID INTERNALDATE)";
    const BODY_QUERY: &'static str = "BODY[TEXT]";
    const DOMAIN: &'static str = "imap.gmail.com";
    const PORT: u16 = 993;

    pub fn new(username: String, pass: String) -> Self {
        Self {
            session: None,
            credentials: Credentials { username, pass },
        }
    }

    pub fn get_last_emails(&mut self) -> Result<(Vec<EmailHeader>, Offset), GmailError> {
        let (_, inbox) = self.get_session()?;

        let total_emails = inbox.exists;
        //We add 1 as the sequence is not inclusive with the upper limit.
        //By doing this, when loading messages for the first time, we also
        //get the last email
        let (range, offset) = generate_sequence(total_emails + 1, PAGE_SIZE as u32);
        Ok((self.get_emails_by_sequence(range)?, offset))
    }

    pub fn get_more_emails(
        &mut self,
        offset: Offset,
    ) -> Result<(Vec<EmailHeader>, Offset), GmailError> {
        let (range, offset) = generate_sequence(offset, PAGE_SIZE as u32);
        Ok((self.get_emails_by_sequence(range)?, offset))
    }

    pub fn get_email_info(&mut self, uid: Uid) -> Result<String, GmailError> {
        let (session, _) = self.get_session()?;

        let fetch_result = session.uid_fetch(format!("{}", uid), Self::BODY_QUERY)?;

        let fetch_result = fetch_result.get(0).ok_or(GmailError::NotUid(uid))?;
        Ok(parse(fetch_result.text().missing_param("TEXT")?)?)
    }

    pub fn send_email(&mut self, to: &str, subject: &str, body: String) -> Result<(), GmailError> {
        let email = MessageBuilder::new()
            .to(to.parse()?)
            .from(self.credentials.username.parse()?)
            .subject(subject)
            .body(body)
            .unwrap();

        let credentials = auth::Credentials::new(
            self.credentials.username.clone(),
            self.credentials.pass.clone(),
        );

        let mailer = SmtpTransport::relay("smtp.gmail.com")?
            .credentials(credentials)
            .build();

        mailer.send(&email)?;

        Ok(())
    }

    fn login(&mut self) -> Result<(), GmailError> {
        let tls = TlsConnector::builder().build()?;
        let client = imap::connect((Self::DOMAIN, Self::PORT), Self::DOMAIN, &tls)?;
        let mut session = client
            .login(
                self.credentials.username.clone(),
                self.credentials.pass.clone(),
            )
            .map_err(|(e, _)| e)?;

        let inbox = session.select("INBOX")?;
        self.session = Some((session, inbox));

        Ok(())
    }

    fn get_session(&mut self) -> Result<&mut (Session, Mailbox), GmailError> {
        if self.session.is_some() {
            return Ok(self.session.as_mut().unwrap());
        }

        self.login()?;
        Ok(self.session.as_mut().unwrap())
    }

    fn get_emails_by_sequence(&mut self, range: String) -> Result<Vec<EmailHeader>, GmailError> {
        let (session, _) = self.get_session()?;
        let fetch_result = session.fetch(range, Self::HEADER_QUERY)?;

        Ok(fetch_result
            .iter()
            .rev()
            .map(create_email_header)
            .collect::<Result<Vec<EmailHeader>, GmailError>>()?)
    }
}

fn create_email_header(fetch: &imap::types::Fetch) -> Result<EmailHeader, GmailError> {
    let date = fetch
        .internal_date()
        .map(|d| d.to_string())
        .missing_param("INTERNALDATE")?;
    let env = fetch.envelope().missing_param("ENVELOPE")?;
    let uid = fetch.uid.missing_param("UID")?;
    let subject = parse(env.subject.missing_param("SUBJECT")?).unwrap_or("".to_string());
    let from = env
        .from
        .as_ref()
        .missing_param("FROM")?
        .iter()
        .filter_map(parse_address)
        .collect::<Vec<String>>()
        .join(";");

    Ok(EmailHeader {
        from,
        date,
        subject,
        uid,
    })
}

fn parse_address(address: &imap_proto::Address) -> Option<String> {
    let name = parse(address.name.unwrap_or_default()).ok()?;
    let mailbox = parse(address.mailbox.unwrap_or_default()).ok()?;

    Some(format!("{} <{}>", name, mailbox))
}

fn parse(buf: &[u8]) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(buf.to_vec())
}

fn generate_sequence(last: u32, qty: u32) -> (String, Offset) {
    //In case there are less emails than the ones specified, we default to
    //0 as the first element has sequence number 1. We add 1 in general because
    //the API expects the last sequence element and gents qty elements. This
    //means that we need to add 1 after substracting to get exactly qty elements
    let start = last.checked_sub(qty).unwrap_or(1);
    (
        (start..last)
            .map(|n| n.to_string())
            .collect::<Vec<String>>()
            .join(","),
        start,
    )
}
