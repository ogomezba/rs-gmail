use imap::types::Uid;
use lettre::{address::AddressError, transport::smtp::Error};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GmailError {
    #[error("[LOGIN_ERROR]: An error occurred while interacting with the Gmail imap client.")]
    Imap(#[from] imap::Error),
    #[error("[UNEXPECTED_ERROR]: Something went wrong: {0}.")]
    Tls(#[from] native_tls::Error),
    #[error("[UNSPECIFIED_PARAMETER]: Parameter not specified when fetching information: {0}.")]
    UnspecifiedParameter(&'static str),
    #[error("[MISSING_UID]: The Uid that has been specified does not exist in the inbox: {0}.")]
    NotUid(Uid),
    #[error("[PARSING_ERROR]: Error when parsing the body of the message.")]
    Parser(#[from] std::string::FromUtf8Error),
    #[error("[LOCK_ERROR]: An error occurred while obtaining unique access to the Gmail inbox.")]
    Lock,
    #[error("[SMTP]: An error occurred while sending the email.")]
    Smtp(#[from] Error),
    #[error("[ADDRESS_ERROR]: Invalid address format.")]
    Address(#[from] AddressError),
}

pub trait MissingParam<T> {
    fn missing_param(self, param: &'static str) -> Result<T, GmailError>;
}

impl<T> MissingParam<T> for Option<T> {
    fn missing_param(self, param: &'static str) -> Result<T, GmailError> {
        self.ok_or(GmailError::UnspecifiedParameter(param))
    }
}
