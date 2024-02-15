//! # Redis serialization protocol (RESP) //!
//! To communicate with the Redis server, Redis clients use a protocol called
//! Redis Serialization Protocol (RESP). While the protocol was designed specifically
//! for Redis, you can use it for other client-server software projects.

#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error("Incomplete RESP message")]
    IncompleteMessage,
    #[error("Unknown RESP type: {0:?}")]
    UnknownType(char),
}

pub const CRLF: &str = "\r\n";
pub const SIMPLE_STRING_START: char = '+';
pub const BULK_STRING_START: char = '$';
pub const ARRAY_START: char = '*';

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// RESP Simple strings are encoded as a plus (`+`) character,
    /// followed by a string. The string mustn't contain a CR (`\r`)
    /// or LF (`\n`) character and is terminated by CRLF (i.e., `\r\n`).
    ///
    /// Format: `+<data>\r\n`
    SimpleString { data: String },
    /// A bulk string represents a single binary string.
    /// The string can be of any size, but by default, Redis limits it to 512 MB.
    ///
    /// RESP encodes bulk strings in the following way:
    ///
    /// `$<length>\r\n<data>\r\n`
    ///
    /// - The dollar sign (`$`) as the first byte.
    /// - One or more decimal digits as the string's length.
    /// - The CRLF terminator.
    /// - The data.
    /// - A final CRLF.
    ///
    /// So the string "hello" is encoded as follows:
    ///
    /// `$5\r\nhello\r\n`
    ///
    /// The empty string's encoding is:
    ///
    /// `$0\r\n\r\n`
    ///
    BulkString { data: String },
    /// RESP Arrays' encoding uses the following format:
    ///
    /// `*<number-of-elements>\r\n<element-1>...<element-n>`
    ///
    /// - An asterisk (`*`) as the first byte.
    /// - One or more decimal digits as the number of elements in the array.
    /// - The CRLF terminator.
    /// - An additional RESP type for every element of the array.
    ///
    /// Example:
    ///
    /// `*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n`
    Array { tokens: Vec<Token> },
}

impl Token {
    pub fn extract(&self) -> Option<&str> {
        use Token::{Array, BulkString, SimpleString};
        match self {
            SimpleString { data } | BulkString { data } => Some(data),
            Array { .. } => None,
        }
    }
}

impl TryFrom<&str> for Token {
    type Error = ParseError;

    fn try_from(str: &str) -> Result<Self, Self::Error> {
        let str = str.trim_matches('\0');
        let is_array = str.starts_with(ARRAY_START);
        let mut parts = str
            .split(CRLF)
            .filter(|part| !part.is_empty() && !part.starts_with(ARRAY_START))
            .peekable();

        let mut tokens: Vec<Self> = vec![];
        while let Some(str) = parts.next() {
            match str.chars().next().ok_or(ParseError::IncompleteMessage)? {
                BULK_STRING_START => {
                    tokens.push(Self::BulkString {
                        // HACK: Clippy suggested some dereference magic for a faster `to_string()`.
                        data: (*parts.peek().ok_or(ParseError::IncompleteMessage)?).to_string(),
                    });
                    parts.next(); // Don't handle the bulk string twice.
                }
                SIMPLE_STRING_START => tokens.push(Self::SimpleString {
                    data: str[1..].to_string(),
                }),
                unknown_type => return Err(ParseError::UnknownType(unknown_type)),
            }
        }

        match (tokens.len(), is_array) {
            (1.., true) | (0, _) => Ok(Self::Array { tokens }),
            (1.., false) => Ok(tokens.first().expect("").clone()),
            (_, _) => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Token;

    #[test]
    fn simple_string_pong() {
        let token = Token::try_from("+PONG\r\n").unwrap();
        assert_eq!(
            token,
            Token::SimpleString {
                data: String::from("PONG")
            }
        );
    }

    #[test]
    fn simple_string_ok() {
        let token = Token::try_from("+OK\r\n").unwrap();
        assert_eq!(
            token,
            Token::SimpleString {
                data: String::from("OK")
            }
        );
    }

    #[test]
    fn bulk_string_hello() {
        let token = Token::try_from("$5\r\nhello\r\n").unwrap();
        assert_eq!(
            token,
            Token::BulkString {
                data: String::from("hello")
            }
        );
    }

    #[test]
    fn bulk_string_array() {
        let token = Token::try_from("*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n").unwrap();
        assert_eq!(
            token,
            Token::Array {
                tokens: vec![
                    Token::BulkString {
                        data: String::from("ECHO")
                    },
                    Token::BulkString {
                        data: String::from("hey")
                    }
                ]
            }
        );
    }

    #[test]
    fn mixed_string_array() {
        let token = Token::try_from("*2\r\n$4\r\nECHO\r\n+hey\r\n").unwrap();
        assert_eq!(
            token,
            Token::Array {
                tokens: vec![
                    Token::BulkString {
                        data: String::from("ECHO")
                    },
                    Token::SimpleString {
                        data: String::from("hey")
                    }
                ]
            }
        );
    }

    #[test]
    fn sinle_element_array() {
        let token = Token::try_from("*1\r\n$4\r\nECHO\r\n").unwrap();
        assert_eq!(
            token,
            Token::Array {
                tokens: vec![Token::BulkString {
                    data: String::from("ECHO")
                },]
            }
        );
    }
}
