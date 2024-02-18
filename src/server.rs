//! # Redis server, handles clients and interacts with the [`Database`].

use crate::command::{self, Command};
use crate::config::Config;
use crate::database::{Database, Error};
use crate::resp::{Token, CRLF, SIMPLE_STRING_START};
use std::convert::Infallible;
use std::{io, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::instrument;

/// The address and port on which the [`Server`] listens.
pub const LISTEN_ADDR: &str = "127.0.0.1:6379";

/// The Redis server.
///
/// Owns a [`Database`] (protected by an `Arc<Mutex>`) and a [`TcpListener`].
#[derive(Debug)]
pub struct Server {
    pub db: Arc<Mutex<Database>>,
    listener: TcpListener,
    config: Config,
}

impl Server {
    /// Construct a new [`Server`].
    pub async fn new(config: Config) -> io::Result<Self> {
        Ok(Self {
            db: Arc::new(Mutex::new(Database::new())),
            listener: TcpListener::bind(LISTEN_ADDR).await?,
            config,
        })
    }

    /// Handle all incoming connections.
    ///
    /// This function runs indefinitely and requires `&self` (The [`Server`])
    /// To outlive `'static`. It only returns if an error occurs.
    /// See `main.rs` for an example initialization.
    #[instrument(name = "server", skip(self))]
    pub async fn run(&'static self) -> anyhow::Result<Infallible> {
        loop {
            let (mut socket, _) = self.listener.accept().await?;
            tokio::spawn(async move {
                match self.handle_client(&mut socket).await {
                    Ok(_) => {}
                    Err(err) => tracing::error!("{err}"),
                }
            });
        }
    }

    /// Execute a [`Command`] on the contained [`Database`].
    #[instrument(skip(self, stream))]
    async fn exec(&self, command: Command, stream: &mut TcpStream) -> anyhow::Result<()> {
        match command {
            Command::Ping => {
                let _ = stream
                    .write(format!("{SIMPLE_STRING_START}PONG{CRLF}").as_bytes())
                    .await?;
            }
            Command::Echo { message } => {
                let _ = stream
                    .write((format!("{SIMPLE_STRING_START}{message}{CRLF}")).as_bytes())
                    .await?;
            }
            Command::Set { key, value } => {
                self.db.lock().await.set(key, value);
                let _ = stream
                    .write((format!("{SIMPLE_STRING_START}OK{CRLF}")).as_bytes())
                    .await?;
            }
            Command::Get { key } => {
                let db = self.db.lock().await;
                let response: String = match db.get(&key) {
                    Ok(value) => format!("+{}", value.data),
                    Err(Error::KeyNotFound) => "-Key not found".to_string(),
                    Err(Error::Expired) => "$-1".to_string(),
                };
                let _ = stream.write(format!("{response}{CRLF}").as_bytes()).await?;
            }
            Command::ConfigGet { key } => {
                let response = Token::Array {
                    tokens: vec![
                        Token::BulkString { data: key.clone() },
                        Token::BulkString {
                            data: match key.as_str() {
                                "dir" => self.config.dir.to_string_lossy().to_string(),
                                "filename" => self.config.dbfilename.to_string_lossy().to_string(),
                                _ => return Err(command::ParseError::MissingArgument.into()),
                            },
                        },
                    ],
                };
                let _ = stream.write(response.to_string().as_bytes()).await?;
            }
        }

        Ok(())
    }

    /// Interpret and handle RESP-encoded commands from `stream`.
    ///
    /// # Errors
    ///
    /// This function only errors out if the incoming RESP-encoded stream is invalid,
    /// contains unknown commands, or wrong/missing arguments to commands.
    async fn handle_client(&self, stream: &mut TcpStream) -> anyhow::Result<()> {
        let mut request = [0; 512];

        // `stream.read()` reads until a newline, so lets
        // run it in a loop to read everything line-by-line.
        while let Ok(read_bytes) = stream.read(&mut request).await {
            // Having nothing to read is not an error, it's an Ok(0).
            // Without this, the loop will run until an error occurs.
            if read_bytes == 0 {
                break;
            }

            // If we actually read something meaningful, respond to it.
            let string = String::from_utf8(request.to_vec())?;
            let syntax = Token::try_from(string.as_str())?;
            let command = Command::try_from(syntax)?;

            self.exec(command, stream).await?;
        }

        Ok(())
    }
}
