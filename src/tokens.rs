use serde::Deserialize;
use std::fmt::Display;

#[non_exhaustive]
#[derive(Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Token {
    /// %C: Identifies the connection endpoints, containing four space-separated
    /// values:  client  address, client port number, server address, and server port number.
    ConnectionEndpoints,

    /// %D: The routing domain in which the incoming connection was received.
    RoutingDomain,

    /// %F: The fingerprint of the CA key.
    FingerPrintCaKey,

    /// %f: The fingerprint of the key or certificate.
    FingerPrintCaKeyOrCert,

    /// %h: The home directory of the user.
    HomeDirUser,

    /// %i: The key ID in the certificate.
    KeyIdCert,

    /// %K: The base64-encoded CA key.
    Base64EncodedCaKey,

    /// %k: The base64-encoded key or certificate for authentication.
    Base64EncodedAuthKeyOrCert,

    /// %s: The serial number of the certificate.
    CertificateSerialNumber,

    /// %T: The type of the CA key.
    CaKeyType,

    /// %t: The key or certificate type.
    CertKeyType,

    /// %U: The numeric user ID of the target user.
    UserId,

    /// %u: The username.
    UserName,
}

impl<'a> TryFrom<&'a str> for Token {
    type Error = ();

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let token = match value {
            "%C" => Self::ConnectionEndpoints,
            "%D" => Self::RoutingDomain,
            "%F" => Self::FingerPrintCaKey,
            "%f" => Self::FingerPrintCaKeyOrCert,
            "%h" => Self::HomeDirUser,
            "%i" => Self::KeyIdCert,
            "%K" => Self::Base64EncodedCaKey,
            "%k" => Self::Base64EncodedAuthKeyOrCert,
            "%s" => Self::CertificateSerialNumber,
            "%T" => Self::CaKeyType,
            "%t" => Self::CertKeyType,
            "%U" => Self::UserId,
            "%u" => Self::UserName,
            _ => return Err(()),
        };

        Ok(token)
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let token = match self {
            Self::ConnectionEndpoints => "%C",
            Self::RoutingDomain => "%D",
            Self::FingerPrintCaKey => "%F",
            Self::FingerPrintCaKeyOrCert => "%f",
            Self::HomeDirUser => "%h",
            Self::KeyIdCert => "%i",
            Self::Base64EncodedCaKey => "%K",
            Self::Base64EncodedAuthKeyOrCert => "%k",
            Self::CertificateSerialNumber => "%s",
            Self::CaKeyType => "%T",
            Self::CertKeyType => "%t",
            Self::UserId => "%U",
            Self::UserName => "%u",
        };

        write!(f, "{token}")
    }
}
