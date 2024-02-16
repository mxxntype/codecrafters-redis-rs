//! # Command interpretation and handling.

use crate::database::Value;
use crate::resp::{Token, CRLF, SIMPLE_STRING_START};
use const_format::concatcp;
use std::time::Duration;

pub const PONG_RESPONSE: &str = concatcp!(SIMPLE_STRING_START, "PONG", CRLF);

#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error("Unknown command: {0:?}")]
    UnknownCommand(String),
    #[error("Missing command")]
    MissingCommand,
    #[error("Missing command argument")]
    MissingArgument,
    #[error("Wrong command argument")]
    WrongArgument,
}

/// Known commands that the server can respond to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// The server should reply with [`PONG_RESPONSE`].
    Ping,
    /// The server should repeat the `message`.
    Echo { message: String },
    /// Set key to hold the string value.
    ///
    /// If key already holds a value, it is overwritten, regardless of its type.
    /// Any previous TTL associated with the key is discarded on successful operation.
    Set { key: String, value: Value },
    /// Get the value of key.
    ///
    /// If the key does not exist the special value `nil` is returned.
    /// An error is returned if the value stored at `key` is not a string,
    /// because `GET` only handles string values.
    Get { key: String },
}

impl TryFrom<Token> for Command {
    type Error = ParseError;

    fn try_from(tokens: Token) -> Result<Self, Self::Error> {
        use ParseError::{MissingArgument, MissingCommand, UnknownCommand, WrongArgument};
        use Token::{Array, BulkString, SimpleString};
        match tokens {
            SimpleString { data } | BulkString { data } => match data.as_str() {
                "ping" => Ok(Self::Ping),
                _ => Err(UnknownCommand(data)),
            },
            Array { tokens } => {
                let command = tokens
                    .first()
                    .ok_or(MissingCommand)?
                    .extract()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let arg_1 = tokens.get(1).ok_or(MissingArgument).map(Token::extract);
                let arg_2 = tokens.get(2).ok_or(MissingArgument).map(Token::extract);
                let arg_3 = tokens.get(4).and_then(Token::extract);
                match (command.as_str(), arg_1, arg_2, arg_3) {
                    ("ping", _, _, _) => Ok(Self::Ping),
                    ("echo", msg, _, _) => Ok(Self::Echo {
                        message: msg?.ok_or(WrongArgument)?.to_string(),
                    }),
                    ("get", key, _, _) => Ok(Self::Get {
                        key: key?.ok_or(WrongArgument)?.to_string(),
                    }),
                    ("set", key, val, ttl) => {
                        let ttl = ttl.map(|ttl| {
                            let ms = ttl.parse::<u64>().ok();
                            ms.map(Duration::from_millis)
                        });
                        Ok(Self::Set {
                            key: key?.ok_or(WrongArgument)?.to_string(),
                            value: Value::new(
                                val?.ok_or(WrongArgument)?.to_string(),
                                ttl.flatten(),
                            ),
                        })
                    }
                    _ => Err(UnknownCommand(command)),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Command;
    use crate::{database::Value, resp::Token};

    #[test]
    fn parse_ping() {
        let tokens = Token::try_from("+ping\r\n").unwrap();
        let command = Command::try_from(tokens).unwrap();
        assert_eq!(command, Command::Ping);
    }

    #[test]
    fn parse_echo() {
        let tokens = Token::try_from("*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n").unwrap();
        let command = Command::try_from(tokens).unwrap();
        assert_eq!(
            command,
            Command::Echo {
                message: String::from("hey")
            }
        );
    }

    #[test]
    fn parse_get() {
        let tokens = Token::try_from("*2\r\n$4\r\nGET\r\n$3\r\nfoo\r\n").unwrap();
        let command = Command::try_from(tokens).unwrap();
        assert_eq!(
            command,
            Command::Get {
                key: "foo".to_string(),
            }
        );
    }

    #[test]
    fn parse_set() {
        let tokens = Token::try_from("*3\r\n$4\r\nSET\r\n$3\r\nfoo\r\n+bar\r\n").unwrap();
        let command = Command::try_from(tokens).unwrap();
        assert_eq!(
            command,
            Command::Set {
                key: "foo".to_string(),
                value: Value::without_ttl("bar".to_string())
            }
        );
    }
}
