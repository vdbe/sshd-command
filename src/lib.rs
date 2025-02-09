use std::{
    error::Error,
    fmt::Display,
    fs::File,
    io::{BufReader, Read, Write},
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use frontmatter::{FrontMatter, FrontMatterError};
use serde::{Deserialize, Serialize};

use tera::{Context, Tera};
use tokens::Token;
use uzers::{get_user_by_name, get_user_by_uid};

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

#[derive(Debug)]
pub enum SshdCommandError {
    FrontMatter(FrontMatterError),
}
impl std::error::Error for SshdCommandError {}

impl Display for SshdCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::FrontMatter(err) => {
                write!(f, "{err}")
            }
        }
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
    fn complete(&mut self) -> Result<(), ()> {
        let user = match (self.uid, &self.name) {
            (Some(uid), _) => {
                let user = get_user_by_uid(uid).expect("provided user id doesn't exist");
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
            _ => return Err(()),
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

pub fn main<I: Iterator<Item = String>, W: Write>(
    mut args: I,
    writer: W,
) -> Result<(), Box<dyn Error>> {
    let template_path = args.next().ok_or("No template path provided")?;

    let f = File::open(&template_path)?;
    let mut reader = BufReader::new(f);
    let front_matter = FrontMatter::parse(&mut reader)?;

    let mut context = Context::from_value(front_matter.extra_context)?;

    let mut user = User::default();

    for token in front_matter.sshd_command.tokens() {
        match token {
            Token::ConnectionEndpoints => {
                let client_addr =
                    IpAddr::from_str(args.next().ok_or("No client addr arg")?.as_str())?;

                let client_port = u16::from_str(args.next().ok_or("No client port arg")?.as_str())?;

                let client = SocketAddr::new(client_addr, client_port);

                let server_addr =
                    IpAddr::from_str(args.next().ok_or("No server addr arg")?.as_str())?;

                let server_port = u16::from_str(args.next().ok_or("No server port arg")?.as_str())?;

                let server = SocketAddr::new(server_addr, server_port);

                context.insert("client", &client);
                context.insert("server", &server);
            }
            Token::RoutingDomain => todo!(),
            Token::FingerPrintCaKey => todo!(),
            Token::FingerPrintCaKeyOrCert => todo!(),
            Token::HomeDirUser => todo!(),
            Token::KeyIdCert => todo!(),
            Token::Base64EncodedCaKey => todo!(),
            Token::Base64EncodedAuthKeyOrCert => todo!(),
            Token::CertificateSerialNumber => todo!(),
            Token::CaKeyType => todo!(),
            Token::CertKeyType => todo!(),
            Token::UserId => {
                let uid = u32::from_str(args.next().ok_or("No user id arg")?.as_str())?;
                user.uid = Some(uid);
            }
            Token::UserName => {
                let uname = args.next().ok_or("No user name arg")?;
                user.name = Some(uname);
            }
        }
    }

    if front_matter.sshd_command.complete_user {
        user.complete().map_err(|()| "Failed to complete user")?;
    }
    context.insert("user", &user);

    if front_matter.sshd_command.hostname {
        context.insert(
            "hostname",
            hostname::get()?
                .to_str()
                .expect("Failed to convert hostname"),
        );
    }

    let mut tera = Tera::default();

    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;

    tera.add_raw_template(&template_path, &buf)?;
    tera.render_to(&template_path, &context, writer)?;

    Ok(())
}
