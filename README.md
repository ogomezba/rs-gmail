# rs-gmail

Basic implementation of a Rust client for the Gmail email server.

## Description

The main idea for the client is to expose the basic operations of getting emails
in descending order, getting their contents and send emails.

It is mainly used as the "server-side" of the [nvim-gmail](https://github.com/ogomezba/nvim-gmail)
Neovim plugin.

## Usage

The client offers 4 basic methods:

```Rust
use rs_gmail::{inbox::GmailInbox, GmailError};

fn main() -> Result<(), GmailError> {
    //Create a client
    let mut client = GmailInbox::new("Your username".to_string(), "Your pass".to_string());

    //Get the last 20 emails of the Gmail Inbox in descending date order
    let (email_headers, offset) = client.get_last_emails()?;

    //Get the body of the last received email
    let last_email_body = client.get_email_info(email_headers[0].uid)?;

    //Get the next 20 emails of the inbox
    let (more_email_headers, new_offset) = client.get_more_emails(offset)?;

    Ok(())
}
```

**NOTE**: In case 2-Step Verification is enabled for Gmail, you need to generate an
[application password](https://support.google.com/mail/answer/185833?hl) and use
that as your password when using `GmailInbox::new()`.
