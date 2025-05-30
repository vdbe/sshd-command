use std::io::{BufRead, BufReader, Read};

use semver::{Version, VersionReq};
use serde::Deserialize;
use thiserror::Error;

use crate::{
    crate_version, Command, CommandTrait, KeysCommand, PrincipalCommand,
    SshdCommandError, Token,
};

#[derive(Error, Debug)]
pub enum FrontMatterError {
    #[error("first line must be '---'")]
    InvalidFirstLine,

    #[error(
        "missing end separator for frontmatter, template does not contain a second '---' line"
    )]
    MissingEndSeparator,

    #[error(
        "template requires sshd-command version {1}, but you are running {0}"
    )]
    InvalidVersion(Version, VersionReq),

    #[error("{1} is not a valid token for {0}")]
    UnsupportedToken(Command, Token),

    #[error("parse error: {0}")]
    ParseError(Box<dyn std::error::Error>),
}

#[derive(Deserialize, PartialEq, Eq, Debug, Default)]
pub struct FrontMatter {
    pub(crate) sshd_command: FrontMatterSshdCommand,

    #[serde(flatten)]
    pub(crate) extra_context: tera::Value,
}

#[derive(Deserialize, PartialEq, Eq, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct FrontMatterSshdCommand {
    command: Command,
    tokens: FrontMatterTokens,
    version: VersionReq,

    #[serde(default = "bool::default")]
    pub(crate) complete_user: bool,

    #[serde(default = "bool::default")]
    pub(crate) hostname: bool,
}

#[derive(PartialEq, Eq, Debug, Default)]
pub struct FrontMatterTokens(pub(crate) Box<[Token]>);

impl FrontMatter {
    const SEPARATOR: &'static str = "---";

    #[must_use]
    pub const fn tokens(&self) -> &[Token] {
        self.sshd_command.tokens()
    }

    /// # Errors
    ///
    /// Will return `Err` on invalid front matter.
    ///
    /// # Panics
    ///
    /// Will panic when [`crate_version`] panics.
    pub fn validate(&self) -> Result<(), FrontMatterError> {
        // Check if the version is valid
        let version_req = &self.sshd_command.version;
        let crate_version = crate_version();
        if !version_req.matches(&crate_version) {
            return Err(FrontMatterError::InvalidVersion(
                crate_version,
                version_req.clone(),
            ));
        }

        // Check if all tokens are supported by the command
        let command = self.sshd_command.command;
        let tokens = &self.sshd_command.tokens.0;
        let token_validation = match command {
            Command::Keys => KeysCommand::validate_tokens(tokens),
            Command::Principals => PrincipalCommand::validate_tokens(tokens),
        };

        token_validation.map_err(|token| {
            FrontMatterError::UnsupportedToken(command, token)
        })?;

        // If complete_user check if the required token(s) are provided
        if self.sshd_command.complete_user.then(|| {
            tokens
                .iter()
                .any(|&t| matches!(t, Token::UserId | Token::UserName))
        }) == Some(false)
        {
            return Err(FrontMatterError::ParseError(
                "`%U` or`%u` token required for `complete_user = true`".into(),
            ));
        }

        Ok(())
    }

    /// # Errors
    ///
    /// Will return `Err` when failing to parse the provided front matter.
    ///
    pub fn parse<R: Read>(
        reader: &mut BufReader<R>,
    ) -> Result<Self, FrontMatterError> {
        let mut buf = String::new();
        let mut buf_len;

        // Check if first line is front matter start
        reader
            .read_line(&mut buf)
            .map_err(|err| FrontMatterError::ParseError(Box::new(err)))?;
        if !buf.trim_end().eq(Self::SEPARATOR) {
            return Err(FrontMatterError::InvalidFirstLine);
        }

        // Read front matter into `buf` and verify front matter end is present
        buf_len = buf.len();
        while reader.read_line(&mut buf).unwrap_or(0) != 0 {
            if buf[buf_len..].trim_end().eq(Self::SEPARATOR) {
                // Reached end of frontmatter
                let front_matter_str = &buf[..buf_len];
                let front_matter: Self =
                    serde_yaml::from_str(front_matter_str).map_err(|err| {
                        FrontMatterError::ParseError(Box::new(err))
                    })?;

                return Ok(front_matter);
            }
            buf_len = buf.len();
        }

        Err(FrontMatterError::MissingEndSeparator)
    }
}

impl FrontMatterSshdCommand {
    pub(crate) const fn tokens(&self) -> &[Token] {
        &self.tokens.0
    }
}

impl From<FrontMatterError> for SshdCommandError {
    fn from(value: FrontMatterError) -> Self {
        Self::FrontMatter(value)
    }
}

mod _serde {
    use core::fmt;

    use serde::{de::Visitor, Deserialize};

    use super::{FrontMatterTokens, Token};

    struct FrontMatterTokensVisitor;

    impl Visitor<'_> for FrontMatterTokensVisitor {
        type Value = Box<[Token]>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a space separated list of sshd_config tokens, see sshd_config(5) for all valid tokens.")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let tokens: Result<Box<[Token]>, _> =
                v.split_whitespace().map(Token::try_from).collect();

            tokens.map_or_else(
                |invalid_token| {
                    Err(serde::de::Error::invalid_type(
                        // TODO: Add better unexpected msg
                        serde::de::Unexpected::Str(invalid_token),
                        &self,
                    ))
                },
                |tokens| Ok(tokens),
            )
        }
    }

    impl<'de> Deserialize<'de> for FrontMatterTokens {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(Self(
                deserializer.deserialize_str(FrontMatterTokensVisitor)?,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use core::panic;
    use std::str::FromStr;

    use super::*;

    fn update_version(
        version: &Version,
        delta_major: i64,
        delta_minor: i64,
        delta_patch: i64,
    ) -> Option<Version> {
        let major = version.major.checked_add_signed(delta_major)?;
        let minor = version.minor.checked_add_signed(delta_minor)?;
        let patch = version.patch.checked_add_signed(delta_patch)?;

        Some(Version::new(major, minor, patch))
    }

    #[test]
    fn check_parse() {
        let template = format!(
            r"---
sshd_command:
    version: {}
    command: principals
    tokens: '%U %u'
---
        ",
            crate_version()
        );

        let mut reader = BufReader::new(template.as_bytes());
        let front_matter = FrontMatter::parse(&mut reader);

        assert!(front_matter.is_ok());
        let front_matter = front_matter.unwrap();
        assert!(front_matter.validate().is_ok());

        let front_matter_expected = FrontMatter {
            sshd_command: FrontMatterSshdCommand {
                command: Command::Principals,
                tokens: FrontMatterTokens(Box::new([
                    Token::UserId,
                    Token::UserName,
                ])),
                version: VersionReq::parse(&crate_version().to_string())
                    .expect("Failed to parse crate version as `VersionReq`"),
                complete_user: false,
                hostname: false,
            },
            extra_context: tera::Value::Object(tera::Map::new()),
        };
        assert_eq!(front_matter, front_matter_expected);
    }

    #[test]
    fn check_parse_full() {
        let template = format!(
            r"---
sshd_command:
    version: {}
    command: principals
    tokens: '%U %u'
    complete_user: true
    hostname: true
search_domains:
    - home.arpa
    - local
---
        ",
            crate_version()
        );

        let mut reader = BufReader::new(template.as_bytes());
        let front_matter = FrontMatter::parse(&mut reader);

        assert!(front_matter.is_ok());
        let front_matter = front_matter.unwrap();
        assert!(front_matter.validate().is_ok());

        let mut extra_content = tera::Map::new();
        let _ = extra_content.insert(
            "search_domains".into(),
            vec!["home.arpa", "local"].into(),
        );

        let front_matter_expected = FrontMatter {
            sshd_command: FrontMatterSshdCommand {
                command: Command::Principals,
                tokens: FrontMatterTokens(Box::new([
                    Token::UserId,
                    Token::UserName,
                ])),
                version: VersionReq::parse(&crate_version().to_string())
                    .expect("Failed to parse crate version as `VersionReq`"),
                complete_user: true,
                hostname: true,
            },
            extra_context: tera::Value::Object(extra_content),
        };
        assert_eq!(front_matter, front_matter_expected);
    }

    #[test]
    fn check_parse_json() {
        let template = format!(
            r"---
sshd_command:
    version: {}
    command: principals
    tokens: '%U %u'
    complete_user: true
    hostname: true
search_domains:
    - home.arpa
    - local
---
        ",
            crate_version()
        );
        let template_json = format!(
            r#"---
{{
  "sshd_command": {{
    "version": "{}",
    "command": "principals",
    "tokens": "%U %u",
    "complete_user": true,
    "hostname": true
  }},
  "search_domains": [
    "home.arpa",
    "local"
  ]
}}
---
        "#,
            crate_version()
        );

        let mut reader = BufReader::new(template.as_bytes());
        let front_matter = FrontMatter::parse(&mut reader).unwrap();

        let mut reader = BufReader::new(template_json.as_bytes());
        let front_matter_json = FrontMatter::parse(&mut reader).unwrap();
        assert_eq!(front_matter, front_matter_json);
    }

    #[test]
    fn check_parse_next_line() {
        let template = format!(
            r"---
sshd_command:
    version: {}
    command: principals
    tokens: '%U %u'
---
next-line
        ",
            crate_version()
        );

        let mut reader = BufReader::new(template.as_bytes());
        let front_matter = FrontMatter::parse(&mut reader);

        assert!(front_matter.is_ok());

        let next_line = reader.lines().next();
        assert!(matches!(next_line, Some(Ok(_))));

        let next_line = next_line.unwrap().unwrap();
        assert_eq!(next_line, "next-line");
    }

    #[test]
    fn check_parse_invalid_first_line() {
        let template = format!(
            r"
sshd_command:
    version: {}
    command: principals
    tokens: '%U %u'
---
        ",
            crate_version()
        );

        let mut reader = BufReader::new(template.as_bytes());
        let front_matter = FrontMatter::parse(&mut reader);

        assert!(matches!(
            front_matter,
            Err(FrontMatterError::InvalidFirstLine)
        ));
    }

    #[test]
    fn check_parse_missing_end_separator() {
        let template = format!(
            r"---
sshd_command:
    version: {}
    command: principals
    tokens: '%U %u'
        ",
            crate_version()
        );

        let mut reader = BufReader::new(template.as_bytes());
        let front_matter = FrontMatter::parse(&mut reader);

        assert!(matches!(
            front_matter,
            Err(FrontMatterError::MissingEndSeparator)
        ));
    }

    #[test]
    fn check_parse_unknown_token() {
        let template = format!(
            r"---
sshd_command:
    version: {}
    command: principals
    tokens: '%U %u %invalid'
---
        ",
            crate_version()
        );

        let mut reader = BufReader::new(template.as_bytes());
        let front_matter = FrontMatter::parse(&mut reader);

        assert!(matches!(front_matter, Err(FrontMatterError::ParseError(_))));

        let error = front_matter.err().unwrap().to_string();
        assert!(error.contains("%invalid"));
    }

    #[test]
    fn check_parse_invalid_version() {
        let template = r"---
sshd_command:
    version: 9999.9999.9999
    command: principals
    tokens: '%U %u'
---
        ";

        let mut reader = BufReader::new(template.as_bytes());
        let front_matter = FrontMatter::parse(&mut reader);

        assert!(front_matter.is_ok());
        let front_matter = front_matter.unwrap().validate();

        assert!(matches!(
            front_matter,
            Err(FrontMatterError::InvalidVersion(_, _))
        ));
    }

    #[test]
    fn check_parse_missing_option() {
        let template = format!(
            r"---
sshd_command:
    version: {}
    # command: principals
    tokens: '%U %u'
---
        ",
            crate_version()
        );

        let mut reader = BufReader::new(template.as_bytes());
        let front_matter = FrontMatter::parse(&mut reader);

        assert!(matches!(front_matter, Err(FrontMatterError::ParseError(_))));

        let error = front_matter.err().unwrap().to_string().to_lowercase();
        assert!(error.contains("missing field"));
        assert!(error.contains("command"));
    }

    #[test]
    fn check_parse_out_of_order() {
        let template = format!(
            r"---
sshd_command:
    tokens: '%U %u'
    version: {}
    command: principals
---
        ",
            crate_version()
        );

        let mut reader = BufReader::new(template.as_bytes());
        let front_matter = FrontMatter::parse(&mut reader);
        assert!(front_matter.is_ok());
    }

    #[test]
    fn check_validate_default() {
        assert!(FrontMatter::default().validate().is_ok());
    }

    #[test]
    fn check_complete_user() {
        let mut front_matter = FrontMatter::default();
        assert!(front_matter.validate().is_ok());

        front_matter.sshd_command.complete_user = true;
        assert!(matches!(
            front_matter.validate(),
            Err(FrontMatterError::ParseError(_))
        ));

        front_matter.sshd_command.tokens =
            FrontMatterTokens(Box::new([Token::UserName]));
        assert!(front_matter.validate().is_ok());
        front_matter.sshd_command.tokens =
            FrontMatterTokens(Box::new([Token::UserId]));
        assert!(front_matter.validate().is_ok());

        front_matter.sshd_command.tokens =
            FrontMatterTokens(Box::new([Token::HomeDirUser]));
        assert!(matches!(
            front_matter.validate(),
            Err(FrontMatterError::ParseError(_))
        ));
    }

    #[test]
    fn check_validate_supported_tokens() {
        let mut front_matter = FrontMatter::default();

        {
            front_matter.sshd_command.command = Command::Keys;
            front_matter.sshd_command.tokens = FrontMatterTokens(Box::new([
                Token::ConnectionEndpoints,
                Token::UserName,
            ]));

            assert!(front_matter.validate().is_ok());

            front_matter.sshd_command.tokens = FrontMatterTokens(Box::new([
                Token::ConnectionEndpoints,
                Token::CaKeyType,
                Token::UserName,
            ]));

            assert!(matches!(
                front_matter.validate(),
                Err(FrontMatterError::UnsupportedToken(
                    Command::Keys,
                    Token::CaKeyType,
                ))
            ));
        }
        {
            // `Command::Principals` supports everything
        }
    }

    #[test]
    fn check_validate_required_version() {
        let crate_version = crate_version();
        let mut front_matter = FrontMatter::default();

        if let Some(required_version) = update_version(&crate_version, 0, 0, 0)
        {
            front_matter.sshd_command.version =
                VersionReq::from_str(&required_version.to_string()).unwrap();
            assert!(front_matter.validate().is_ok());
        }

        if let Some(required_version) = update_version(&crate_version, 1, 0, 0)
        {
            front_matter.sshd_command.version =
                VersionReq::from_str(&required_version.to_string()).unwrap();

            if let Err(FrontMatterError::InvalidVersion(_, _)) =
                front_matter.validate()
            {
            } else {
                panic!();
            }
        }

        if let Some(required_version) =
            update_version(&crate_version, -1, 0, 0)
        {
            front_matter.sshd_command.version =
                VersionReq::from_str(&required_version.to_string()).unwrap();

            if let Err(FrontMatterError::InvalidVersion(_, _)) =
                front_matter.validate()
            {
            } else {
                panic!();
            }
        }
        if let Some(required_version) = update_version(&crate_version, 0, 1, 0)
        {
            front_matter.sshd_command.version =
                VersionReq::from_str(&required_version.to_string()).unwrap();

            if let Err(FrontMatterError::InvalidVersion(_, _)) =
                front_matter.validate()
            {
            } else {
                panic!();
            }
        }

        if let Some(required_version) =
            update_version(&crate_version, 0, -1, 0)
        {
            if required_version.major != 0 {
                front_matter.sshd_command.version =
                    VersionReq::from_str(&required_version.to_string())
                        .unwrap();

                if let Err(FrontMatterError::InvalidVersion(_, _)) =
                    front_matter.validate()
                {
                    panic!();
                }
            }
        }

        if let Some(required_version) = update_version(&crate_version, 0, 0, 1)
        {
            front_matter.sshd_command.version =
                VersionReq::from_str(&required_version.to_string()).unwrap();

            if let Err(FrontMatterError::InvalidVersion(_, _)) =
                front_matter.validate()
            {
            } else {
                panic!();
            }
        }

        if let Some(required_version) =
            update_version(&crate_version, 0, 0, -1)
        {
            front_matter.sshd_command.version =
                VersionReq::from_str(&required_version.to_string()).unwrap();

            if let Err(FrontMatterError::InvalidVersion(_, _)) =
                front_matter.validate()
            {
                panic!();
            }
        }
    }
}
