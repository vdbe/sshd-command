/// Parse arguments from iterator and return correct error types if necessary.
///
/// # Macro Branches
///
/// - (0) `arg!($args, $token)`
///   Take argument from iterator, If no argument exists returns a
///   `Err(SshdCommandError::MissingTokenArgument($token))`.
///
/// - (1) `arg!($args, $ty, $token)`
///   Take argument from iterator with (0) and parses it into type `$ty` with
///   `<$ty>::from_str`.
///    On failure return `Err(SshdCommandError::InvalidArgumentToken($token))`.
///
/// # Examples
///
/// ```ignore
/// # #[macro_use] extern crate sshd_command;
/// # use crate::{error::SshdCommandError, tokens::Token};
/// // Get an argument without parsing
/// let username = next_arg!(token, Token::UserName);
///
/// // Get an argument and parse it into a `u16`
/// let port  = next_arg!(token, u16, Token::ConnectionEndPoints);
/// let port: u16 = next_arg!(token, _, Token::ConnectionEndPoints);
/// ```
macro_rules! next_arg {
    // (0)
    ($args:expr, $token:expr) => {{
        $args.next().ok_or(
            crate::error::SshdCommandError::MissingTokenArgument($token),
        )?
    }};

    // (1)
    ($args:expr, $ty:ty, $token:expr) => {{
        {
            let arg = next_arg!($args, $token);
            <$ty as std::str::FromStr>::from_str(&arg).map_err(|_| {
                crate::error::SshdCommandError::InvalidTokenArgument(
                    $token,
                    arg.clone(),
                )
            })?
        }
    }};
}

macro_rules! define_tokens {
    (
        $(#[$enum_attr:meta])*
        // explicit separator between doc comment for token and first variant
        // doc comment
        ;

        $(
            $(#[$meta:meta])*
            $variant:ident => $variant_str:expr;
        )+
    ) => {
        $(#[$enum_attr])*
        pub enum Token {
            $(
                $(#[$meta])*
                $variant,
            )+
        }

        impl Token {
            const fn as_str(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant => $variant_str,
                    )+
                }
            }
        }

        impl std::fmt::Display for Token {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl<'a> std::convert::TryFrom<&'a str> for Token {
            type Error = &'a str;
            fn try_from(s: &'a str) -> Result<Self, Self::Error> {
                let token = match s {
                $(
                    $variant_str => Self::$variant,
                )+
                    _ => return Err(s),
                };

                Ok(token)
            }
        }
    }
}

pub(crate) use define_tokens;
pub(crate) use next_arg;
