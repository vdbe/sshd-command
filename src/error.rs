use thiserror::Error;

use crate::{frontmatter::FrontMatterError, Token};

#[derive(Error, Debug)]
pub enum SshdCommandError {
    #[error("front matter: {0}")]
    FrontMatter(FrontMatterError),

    #[error("token {0} has missing argument(s)")]
    MissingTokenArgument(Token),

    #[error("token {0} has invalid argument: {1}")]
    InvalidTokenArgument(Token, String),

    #[error("tera")]
    Tera(#[from] tera::Error),

    #[error("general error")]
    Unknown(Box<dyn std::error::Error>),
}

impl From<&str> for SshdCommandError {
    fn from(value: &str) -> Self {
        Self::Unknown(value.into())
    }
}
// impl From<tera::Error> for SshdCommandError {
//     fn from(value: tera::Error) -> Self {
//         Self::Tera(value)
//     }
// }
