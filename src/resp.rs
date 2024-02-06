//! # Redis serialization protocol (RESP)
//!
//! To communicate with the Redis server, Redis clients use a protocol called
//! Redis Serialization Protocol (RESP). While the protocol was designed specifically
//! for Redis, you can use it for other client-server software projects.

use std::io::Error;

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

impl TryFrom<&str> for Token {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = value.replace('\0', "");
        let err = Error::new(std::io::ErrorKind::InvalidInput, "Invalid RESP expression");
        let mut substrings = value[1..].split(CRLF);
        match value.chars().next() {
            Some(SIMPLE_STRING_START) => Ok(Self::SimpleString {
                data: substrings.nth(0).ok_or(err)?.to_owned(),
            }),
            Some(BULK_STRING_START) => Ok(Self::BulkString {
                data: substrings.nth(1).ok_or(err)?.to_owned(),
            }),
            Some(ARRAY_START) => {
                let element_count = substrings
                    .next()
                    .ok_or(err)?
                    .parse::<usize>()
                    .map_err(|_| Error::new(std::io::ErrorKind::InvalidInput, "Invalid length"))?;
                let mut vec = Vec::with_capacity(element_count);

                // TODO: Refactor.
                for str in substrings.filter(|str| !str.is_empty()) {
                    match str.chars().next() {
                        Some(SIMPLE_STRING_START) => vec.push(Self::SimpleString {
                            data: str[1..].to_owned(),
                        }),
                        Some(BULK_STRING_START) => continue,
                        Some(_) => vec.push(Self::BulkString {
                            data: str.to_owned(),
                        }),
                        _ => {}
                    }
                }
                Ok(Self::Array { tokens: vec })
            }
            _ => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Token;

    #[test]
    fn simple_string_pong() {
        let command = "+PONG\r\n";
        let datatype = Token::try_from(command).unwrap();
        assert_eq!(
            datatype,
            Token::SimpleString {
                data: String::from("PONG")
            }
        );
    }

    #[test]
    fn simple_string_ok() {
        let command = "+OK\r\n";
        let token = Token::try_from(command).unwrap();
        assert_eq!(
            token,
            Token::SimpleString {
                data: String::from("OK")
            }
        );
    }

    #[test]
    fn bulk_string_hello() {
        let command = "$5\r\nhello\r\n";
        let bulk_string = Token::try_from(command).unwrap();
        assert_eq!(
            bulk_string,
            Token::BulkString {
                data: String::from("hello")
            }
        );
    }

    #[test]
    fn bulk_string_array() {
        let command = "*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n";
        let array = Token::try_from(command).unwrap();
        assert_eq!(
            array,
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
        let command = "*2\r\n$4\r\nECHO\r\n+hey\r\n";
        let array = Token::try_from(command).unwrap();
        assert_eq!(
            array,
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
}
