use std::{
    fmt::Display,
    io::{BufReader, Read, Write},
};

use macros::define_tokens;
use semver::Version;
use serde::Deserialize;
use tera::Tera;

use context::{build_context, User};
use error::SshdCommandError;
use frontmatter::FrontMatter;

mod context;
mod error;
pub mod frontmatter;
mod macros;

define_tokens! {
    /// All possible tokens as documented in SSHD_CONFIG(5))
    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    ;

    /// %C: Identifies the connection endpoints, containing four space-separated values.
    ConnectionEndpoints => "%C";

    /// %D: The routing domain in which the incoming connection was received.
    RoutingDomain => "%D";

    /// %F: The fingerprint of the CA key.
    FingerPrintCaKey => "%F";

    /// %f: The fingerprint of the key or certificate.
    FingerPrintCaKeyOrCert => "%f";

    /// %h: The home directory of the user.
    HomeDirUser => "%h";

    /// %i: The key ID in the certificate.
    KeyIdCert => "%i";

    /// %K: The base64-encoded CA key.
    Base64EncodedCaKey => "%K";

    /// %k: The base64-encoded key or certificate for authentication.
    Base64EncodedAuthKeyOrCert => "%k";

    /// %s: The serial number of the certificate.
    CertificateSerialNumber => "%s";

    /// %T: The type of the CA key.
    CaKeyType => "%T";

    /// %t: The key or certificate type.
    CertKeyType => "%t";

    /// %U: The numeric user ID of the target user.
    UserId => "%U";

    /// %u: The username.
    UserName => "%u";
}

impl Token {
    #[must_use]
    pub fn get_template_args(tokens: &[Self]) -> Vec<String> {
        let placeholder_tokens: Vec<String> = tokens
            .iter()
            .map(|token| match token {
                Self::ConnectionEndpoints => String::from("::1 22 ::1 41644"),
                Self::RoutingDomain => String::from("127.0.0.1/8"),
                Self::FingerPrintCaKey => String::from("_FingerPrintCaKey_"),
                Self::FingerPrintCaKeyOrCert => {
                    String::from("_FingerPrintCaKeyOrCert_")
                }
                Self::HomeDirUser => String::from("/home/place_holder_user"),
                Self::KeyIdCert => String::from("_KeyIdCert_"),
                Self::Base64EncodedCaKey => {
                    String::from("X0Jhc2U2NEVuY29kZWRDYUtleV8=")
                }
                Self::Base64EncodedAuthKeyOrCert => {
                    String::from("X0Jhc2U2NEVuY29kZWRBdXRoS2V5T3JDZXJ0Xw==")
                }
                Self::CertificateSerialNumber => String::from("0"),
                Self::CaKeyType => String::from("sha2-nistp384"),
                Self::CertKeyType => {
                    String::from("ssh-ed25519-cert-v01@openssh.com")
                }
                Self::UserId => User::get_current_uid().to_string(),
                Self::UserName => User::get_current_name(),
            })
            .collect();

        placeholder_tokens
    }
}

#[derive(Deserialize, PartialEq, Eq, Clone, Copy, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Command {
    #[default]
    Keys,
    Principals,
}

enum KeysCommand {}
enum PrincipalCommand {}

trait CommandTrait {
    fn is_token_supported(token: Token) -> bool
    where
        Self: Sized;

    fn validate_tokens(tokens: &[Token]) -> Result<(), Token>
    where
        Self: Sized,
    {
        tokens
            .iter()
            .find(|&&t| !Self::is_token_supported(t))
            .map_or(Ok(()), |&t| Err(t))
    }
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

/// # Errors
///
/// Will return `Err` on an invalid template.
///
/// # Panics
///
/// Will panic on `OsStr::to_str()` errors.
pub fn render_to<I: Iterator<Item = String>, R: Read>(
    writer: &mut dyn Write,
    args: I,
    template_name: &str,
    template: R,
) -> Result<(), SshdCommandError> {
    let mut reader = BufReader::new(template);
    let front_matter = FrontMatter::parse(&mut reader)?;

    front_matter.validate()?;

    let context = build_context(front_matter, args)?;

    // Read tera template
    let mut buf = String::new();
    reader
        .read_to_string(&mut buf)
        .map_err(|e| SshdCommandError::Unknown(Box::new(e)))?;

    // Load tera template
    let mut tera = Tera::default();
    tera.add_raw_template(template_name, &buf)?;

    // Render tera template
    tera.render_to(template_name, &context, writer)?;

    Ok(())
}

#[inline]
#[must_use]
/// # Panics
///
/// Will panic when failing to parse the current crate version into a
/// [`Version`].
pub fn crate_version() -> Version {
    semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .expect("CARGO_PKG_VERSION is always valid")
}
