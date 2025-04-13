use std::net::{IpAddr, SocketAddr};

use serde::Serialize;
use tera::Context;
use uzers::{
    get_current_uid, get_current_username, get_user_by_name, get_user_by_uid,
};

use crate::{
    error::SshdCommandError, frontmatter::FrontMatter, macros::next_arg, Token,
};

#[derive(Debug, Default, Serialize)]
pub struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    gid: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    groups: Option<Vec<Group>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    uid: Option<u32>,
}

#[derive(Debug, Default, Serialize)]
struct Group {
    gid: u32,
    name: String,
}

impl User {
    fn complete(&mut self) -> Result<(), SshdCommandError> {
        let user = match (self.uid, &self.name) {
            (Some(uid), _) => {
                let user = get_user_by_uid(uid).ok_or(
                    SshdCommandError::InvalidTokenArgument(
                        Token::UserId,
                        uid.to_string(),
                    ),
                )?;

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
                let user = get_user_by_name(&name)
                    .expect("provided user name doesn't exist");
                self.uid = Some(user.uid());
                user
            }
            _ => {
                return Err(SshdCommandError::from("Failed to complete user"))
            }
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

    pub(crate) fn get_current_uid() -> u32 {
        get_current_uid()
    }

    pub(crate) fn get_current_name() -> String {
        get_current_username()
            .unwrap_or_else(|| "unkown".into())
            .to_str()
            .expect("Failed to convert username to a str")
            .to_string()
    }
}

pub fn build_context<I: Iterator<Item = String>>(
    front_matter: FrontMatter,
    mut args: I,
) -> Result<Context, SshdCommandError> {
    let mut context = Context::from_value(front_matter.extra_context)?;

    let mut user = User::default();

    // Loop over and parse passed command line arguments for given `Token`
    for token in front_matter.sshd_command.tokens() {
        match token {
            Token::ConnectionEndpoints => {
                // TODO: report what argument is missing not just the token
                let client_addr: IpAddr =
                    next_arg!(args, _, Token::ConnectionEndpoints);
                let client_port: u16 =
                    next_arg!(args, _, Token::ConnectionEndpoints);

                let client = SocketAddr::new(client_addr, client_port);

                let server_addr: IpAddr =
                    next_arg!(args, _, Token::ConnectionEndpoints);
                let server_port: u16 =
                    next_arg!(args, _, Token::ConnectionEndpoints);

                let server = SocketAddr::new(server_addr, server_port);

                context.insert("client", &client);
                context.insert("server", &server);
            }
            Token::RoutingDomain => unimplemented!(),
            Token::FingerPrintCaKey => unimplemented!(),
            Token::FingerPrintCaKeyOrCert => unimplemented!(),
            Token::HomeDirUser => {
                let home_dir = next_arg!(args, Token::UserName);
                context.insert("home_dir", &home_dir);
            }
            Token::KeyIdCert => {
                let key_id: u32 = next_arg!(args, _, Token::KeyIdCert);
                context.insert("key_id", &key_id);
            }
            Token::Base64EncodedCaKey => unimplemented!(),
            Token::Base64EncodedAuthKeyOrCert => unimplemented!(),
            Token::CertificateSerialNumber => unimplemented!(),
            Token::CaKeyType => unimplemented!(),
            Token::CertKeyType => unimplemented!(),
            Token::UserId => {
                let uid: u32 = next_arg!(args, _, Token::UserId);
                user.uid = Some(uid);
            }
            Token::UserName => {
                let uname = next_arg!(args, Token::UserName);
                user.name = Some(uname);
            }
        }
    }

    // Add additional context
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

    Ok(context)
}
