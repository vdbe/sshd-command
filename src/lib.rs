use std::{
    fmt::Display,
    io::{BufReader, Read, Write},
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use error::SshdCommandError;
use frontmatter::FrontMatter;
use serde::{Deserialize, Serialize};

use tera::{Context, Tera};
use tokens::Token;
use uzers::{get_user_by_name, get_user_by_uid};

mod error;
mod frontmatter;
mod tokens;

#[derive(Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Command {
    Keys,
    Principals,
}

impl Command {
    const fn option_name(self) -> &'static str {
        match self {
            Self::Keys => "AuthorizedKeysCommand",
            Self::Principals => "AuthorizedPrincipalsCommand",
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.option_name())
    }
}

trait CommandTrait {
    fn is_token_supported(token: Token) -> bool
    where
        Self: Sized;

    fn validate_tokens(tokens: &[Token]) -> Result<(), Token>
    where
        Self: Sized,
    {
        if let Some(&unsupported_token) = tokens.iter().find(|&t| !Self::is_token_supported(*t)) {
            Err(unsupported_token)
        } else {
            Ok(())
        }
    }
}

enum PrincipalCommand {}
enum KeysCommand {}

impl CommandTrait for PrincipalCommand {
    fn is_token_supported(token: Token) -> bool {
        use Token as Tk;

        matches!(
            token,
            Tk::ConnectionEndpoints
                | Tk::RoutingDomain
                | Tk::FingerPrintCaKey
                | Tk::FingerPrintCaKeyOrCert
                | Tk::HomeDirUser
                | Tk::KeyIdCert
                | Tk::Base64EncodedCaKey
                | Tk::Base64EncodedAuthKeyOrCert
                | Tk::CertificateSerialNumber
                | Tk::CaKeyType
                | Tk::CertKeyType
                | Tk::UserId
                | Tk::UserName
        )
    }
}

impl CommandTrait for KeysCommand {
    fn is_token_supported(token: Token) -> bool {
        use Token as Tk;

        matches!(
            token,
            Tk::ConnectionEndpoints
                | Tk::RoutingDomain
                | Tk::FingerPrintCaKeyOrCert
                | Tk::HomeDirUser
                | Tk::Base64EncodedAuthKeyOrCert
                | Tk::CertKeyType
                | Tk::UserId
                | Tk::UserName
        )
    }
}

#[derive(Debug, Default, Serialize)]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    uid: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    gid: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    groups: Option<Vec<Group>>,
}

impl User {
    fn complete(&mut self) -> Result<(), SshdCommandError> {
        let user = match (self.uid, &self.name) {
            (Some(uid), _) => {
                let user = get_user_by_uid(uid).ok_or(SshdCommandError::InvalidTokenArgument(
                    Token::UserId,
                    format!("{uid}"),
                ))?;

                if self.name.is_none() {
                    self.name = Some(
                        user.name()
                            .to_str()
                            .expect("Failed to convert username to String")
                            .to_string(),
                    );
                }
                user
            }
            (_, Some(name)) => {
                let user = get_user_by_name(&name).expect("provided user name doesn't exist");
                self.uid = Some(user.uid());
                user
            }
            _ => return Err(SshdCommandError::from("Failed to complete user")),
        };

        self.gid = Some(user.primary_group_id());

        let groups: Vec<Group> = user
            .groups()
            .unwrap_or_else(|| Vec::with_capacity(0))
            .into_iter()
            .map(|group| Group {
                gid: group.gid(),
                name: group
                    .name()
                    .to_str()
                    .expect("Failed to convert group name to String")
                    .to_string(),
            })
            .collect();

        self.groups = Some(groups);

        Ok(())
    }
}

#[derive(Debug, Default, Serialize)]
struct Group {
    gid: u32,
    name: String,
}

// NOTE: Should probably be a macro
#[inline]
fn next_arg<I: Iterator<Item = String>>(
    args: &mut I,
    token: Token,
) -> Result<String, SshdCommandError> {
    args.next()
        .ok_or(SshdCommandError::MissingTokenArgument(token))
}

/// # Errors
///
/// Will return `Err` on an invalid template.
///
/// # Panics
///
/// Will panic on `OsStr::to_str()` errors.
pub fn main<I: Iterator<Item = String>, W: Write, R: Read>(
    mut args: I,
    template_name: &str,
    template: R,
    writer: W,
) -> Result<(), SshdCommandError> {
    let mut reader = BufReader::new(template);
    let front_matter = FrontMatter::parse(&mut reader)?;

    let mut context =
        Context::from_value(front_matter.extra_context).map_err(|_| SshdCommandError::Tera)?;

    let mut user = User::default();

    for token in front_matter.sshd_command.tokens() {
        match token {
            Token::ConnectionEndpoints => {
                // TODO: report what argument is missing not just the token
                let arg = next_arg(&mut args, Token::ConnectionEndpoints)?;
                let client_addr = IpAddr::from_str(&arg).map_err(|_| {
                    SshdCommandError::InvalidTokenArgument(Token::ConnectionEndpoints, arg.clone())
                })?;

                let arg = next_arg(&mut args, Token::ConnectionEndpoints)?;
                let client_port = u16::from_str(&arg).map_err(|_| {
                    SshdCommandError::InvalidTokenArgument(Token::ConnectionEndpoints, arg.clone())
                })?;

                let client = SocketAddr::new(client_addr, client_port);

                let arg = next_arg(&mut args, Token::ConnectionEndpoints)?;
                let server_addr = IpAddr::from_str(&arg).map_err(|_| {
                    SshdCommandError::InvalidTokenArgument(Token::ConnectionEndpoints, arg.clone())
                })?;

                let arg = next_arg(&mut args, Token::ConnectionEndpoints)?;
                let server_port = u16::from_str(&arg).map_err(|_| {
                    SshdCommandError::InvalidTokenArgument(Token::ConnectionEndpoints, arg.clone())
                })?;

                let server = SocketAddr::new(server_addr, server_port);

                context.insert("client", &client);
                context.insert("server", &server);
            }
            Token::RoutingDomain => todo!(),
            Token::FingerPrintCaKey => todo!(),
            Token::FingerPrintCaKeyOrCert => todo!(),
            Token::HomeDirUser => {
                let home_dir = next_arg(&mut args, Token::UserName)?;
                context.insert("home_dir", &home_dir);
            }
            Token::KeyIdCert => {
                let arg = next_arg(&mut args, Token::KeyIdCert)?;
                let key_id = u32::from_str(&arg).map_err(|_| {
                    SshdCommandError::InvalidTokenArgument(Token::KeyIdCert, arg.clone())
                })?;
                context.insert("key_id", &key_id);
            }
            Token::Base64EncodedCaKey => todo!(),
            Token::Base64EncodedAuthKeyOrCert => todo!(),
            Token::CertificateSerialNumber => todo!(),
            Token::CaKeyType => todo!(),
            Token::CertKeyType => todo!(),
            Token::UserId => {
                let arg = next_arg(&mut args, Token::UserId)?;
                let uid = u32::from_str(&arg).map_err(|_| {
                    SshdCommandError::InvalidTokenArgument(Token::UserId, arg.clone())
                })?;
                user.uid = Some(uid);
            }
            Token::UserName => {
                let uname = args
                    .next()
                    .ok_or(SshdCommandError::MissingTokenArgument(Token::UserName))?;
                user.name = Some(uname);
            }
        }
    }

    if front_matter.sshd_command.complete_user {
        user.complete()?;
    }
    context.insert("user", &user);

    if front_matter.sshd_command.hostname {
        context.insert(
            "hostname",
            hostname::get()
                .map_err(|_| "Failed to get hostname")?
                .to_str()
                .expect("Failed to convert hostname"),
        );
    }

    let mut tera = Tera::default();

    let mut buf = String::new();
    reader
        .read_to_string(&mut buf)
        .map_err(|e| SshdCommandError::Unknown(Box::new(e)))?;

    tera.add_raw_template(template_name, &buf)
        .map_err(|_| SshdCommandError::Tera)?;
    tera.render_to(template_name, &context, writer)
        .map_err(|_| SshdCommandError::Tera)?;

    Ok(())
}
