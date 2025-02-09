use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use thiserror::Error;

use semver::{Version, VersionReq};
use serde::Deserialize;

use crate::{
    tokens::Token, Command, CommandTrait, KeysCommand, PrincipalCommand, SshdCommandError,
};

#[derive(Error, Debug)]
pub enum FrontMatterError {
    #[error("first line must be '---'")]
    InvalidFirstLine,
    #[error(
        "missing end separator for frontmatter, template does not contain a second '---' line"
    )]
    MissingEndSeparator,

    #[error("template requires sshd-command version {1}, but you are running {0}")]
    InvalidVersion(Version, VersionReq),
    #[error("{1} is not a valid token for {0}")]
    UnsupportedToken(Command, Token),
    #[error("parse error: {0}")]
    ParseError(Box<dyn std::error::Error>),
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct FrontMatter {
    pub(crate) sshd_command: FrontMatterSshdCommand,

    #[serde(flatten)]
    pub(crate) extra_context: tera::Value,
}
#[derive(Deserialize, PartialEq, Eq, Debug)]
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

#[derive(PartialEq, Eq, Debug)]
pub struct FrontMatterTokens(pub(crate) Box<[Token]>);

impl FrontMatter {
    const SEPARATOR: &'static str = "---";

    pub(crate) fn validate(&self) -> Result<(), FrontMatterError> {
        // Check if the versio is valid
        let version_req = &self.sshd_command.version;
        let crate_version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
            .expect("CARGO_PKG_VERSION is always valid");

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

        token_validation.map_err(|token| FrontMatterError::UnsupportedToken(command, token))?;

        Ok(())
    }

    pub(crate) fn parse(reader: &mut BufReader<File>) -> Result<Self, FrontMatterError> {
        let mut buf = String::new();
        let mut buf_len;
        reader
            .read_line(&mut buf)
            .map_err(|e| FrontMatterError::ParseError(Box::new(e)))?;
        if !buf.trim_end().eq(Self::SEPARATOR) {
            return Err(FrontMatterError::InvalidFirstLine);
        }

        buf_len = buf.len();
        while reader.read_line(&mut buf).unwrap_or(0) != 0 {
            if buf[buf_len..].trim_end().eq(Self::SEPARATOR) {
                // Reached end of frontmatter
                let front_matter_str = &buf[..buf_len];
                let front_matter: Self = serde_yaml::from_str(front_matter_str)
                    .map_err(|e| FrontMatterError::ParseError(Box::new(e)))?;

                front_matter.validate()?;

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

    use crate::tokens::Token;

    use super::FrontMatterTokens;

    struct FrontMatterTokensVisitor;

    impl Visitor<'_> for FrontMatterTokensVisitor {
        type Value = Box<[Token]>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a space seperated list of sshd_config tokens, see sshd_config(5) for all valid tokens.")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let tokens: Result<Box<[Token]>, _> =
                v.split_whitespace().map(Token::try_from).collect();

            tokens.map_or_else(
                |()| {
                    Err(serde::de::Error::invalid_type(
                        serde::de::Unexpected::Str(v),
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
