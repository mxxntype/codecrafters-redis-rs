//! # Command interpretation and handling.

use crate::{
    resp::{Token, SEPARATOR, SIMPLE_STRING_START},
    Value,
};
use const_format::concatcp;
use std::{
    io::{Error, ErrorKind},
    time::{Duration, Instant},
};

pub(crate) const PONG_RESPONSE: &str = concatcp!(SIMPLE_STRING_START, "PONG", SEPARATOR);

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
    type Error = Error;

    // TODO: Refactor this horrible shit.
    fn try_from(value: Token) -> Result<Self, Self::Error> {
        let err = Error::new(ErrorKind::InvalidInput, "Unknown or unimplemented command");
        match value {
            Token::SimpleString { data } | Token::BulkString { data } => {
                match data.to_ascii_lowercase().as_str() {
                    "ping" => Ok(Self::Ping),
                    _ => Err(err),
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
                            _ => Err(err),
                        },
                        "get" => match tokens.get(1) {
                            Some(Token::SimpleString { data } | Token::BulkString { data }) => {
                                Ok(Self::Get { key: data.clone() })
                            }
                            _ => Err(err),
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
                                dbg!(ttl);
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
                                let value = Value {
                                    value: value.to_string(),
                                    ttl,
                                    created: Instant::now(),
                                };
                                dbg!(&value);
                                let command = Self::Set {
                                    key: key.to_string(),
                                    value,
                                };
                                Ok(command)
                            }
                            _ => Err(err),
                        },
                        _ => Err(err),
                    }
                } else {
                    Err(err)
                }
            }
        }
    }
}

impl TryFrom<&str> for Command {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let syntax = Token::try_from(value)?;
        Self::try_from(syntax)
    }
}

#[cfg(test)]
mod tests {
    use crate::resp::Token;

    use super::Command;

    #[test]
    fn parse_ping() {
        let command = "+ping\r\n";
        let syntax = Token::try_from(command).unwrap();
        let command = Command::try_from(syntax).unwrap();
        assert_eq!(command, Command::Ping)
    }

    #[test]
    fn parse_echo() {
        let command = "*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n";
        let syntax = Token::try_from(command).unwrap();
        let command = Command::try_from(syntax).unwrap();
        assert_eq!(
            command,
            Command::Echo {
                message: String::from("hey")
            }
        )
    }
}
