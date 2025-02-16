use crate::macros::define_tokens;

define_tokens! {
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
