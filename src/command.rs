//! # Command interpretation and handling.

use crate::{
    database::Value,
    resp::{Token, CRLF, SIMPLE_STRING_START},
};
use const_format::concatcp;
use std::time::Duration;

pub const PONG_RESPONSE: &str = concatcp!(SIMPLE_STRING_START, "PONG", CRLF);

#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error("Unknown command")]
    UknownCommand,
    #[error("Missing command argument")]
    MissingArgument,
}

/// Known commads that the server can respond to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// The server should reply with [`PONG_RESPONSE`].
    Ping,
    /// The server should repeat the [`message`].
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

    // TODO: Refactor this horrible shit.
    fn try_from(value: Token) -> Result<Self, Self::Error> {
        match value {
            Token::SimpleString { data } | Token::BulkString { data } => {
                match data.to_ascii_lowercase().as_str() {
                    "ping" => Ok(Self::Ping),
                    _ => Err(ParseError::UknownCommand),
                }
            }
            Token::Array { tokens } => {
                if let Some(Token::SimpleString { data } | Token::BulkString { data }) =
                    tokens.first()
                {
                    match data.to_ascii_lowercase().as_str() {
                        "ping" => Ok(Self::Ping),
                        "echo" => match tokens.get(1) {
                            Some(Token::SimpleString { data } | Token::BulkString { data }) => {
                                Ok(Self::Echo {
                                    message: data.clone(),
                                })
                            }
                            _ => Err(ParseError::MissingArgument),
                        },
                        "get" => match tokens.get(1) {
                            Some(Token::SimpleString { data } | Token::BulkString { data }) => {
                                Ok(Self::Get { key: data.clone() })
                            }
                            _ => Err(ParseError::MissingArgument),
                        },
                        "set" => match (tokens.get(1), tokens.get(2), tokens.get(4)) {
                            (
                                Some(
                                    Token::SimpleString { data: key }
                                    | Token::BulkString { data: key },
                                ),
                                Some(
                                    Token::SimpleString { data: value }
                                    | Token::BulkString { data: value },
                                ),
                                ttl,
                            ) => {
                                let ttl = match ttl {
                                    Some(
                                        Token::SimpleString { data: ttl }
                                        | Token::BulkString { data: ttl },
                                    ) => {
                                        let ms = ttl.parse::<u64>().unwrap();
                                        Some(Duration::from_millis(ms))
                                    }
                                    _ => None,
                                };
                                let command = Self::Set {
                                    key: key.to_string(),
                                    value: Value::new(value.to_string(), ttl),
                                };
                                Ok(command)
                            }
                            _ => Err(ParseError::MissingArgument),
                        },
                        _ => Err(ParseError::UknownCommand),
                    }
                } else {
                    Err(ParseError::MissingArgument)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Command;
    use crate::resp::Token;

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
}
