use crate::command::{Command, PONG_RESPONSE};
use crate::database::{Database, Error};
use crate::resp::{Token, CRLF, SIMPLE_STRING_START};
use std::{io, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::instrument;

pub const LISTEN_ADDR: &str = "127.0.0.1:6379";

#[derive(Debug)]
pub struct Server {
    pub db: Arc<Mutex<Database>>,
    listener: TcpListener,
}

impl Server {
    /// Construct a new [`Server`].
    pub async fn new() -> io::Result<Self> {
        Ok(Self {
            db: Arc::new(Mutex::new(Database::new())),
            listener: TcpListener::bind(LISTEN_ADDR).await?,
        })
    }

    /// Handle all incoming connections.
    ///
    /// This function runs indefinetely.
    #[instrument(name = "server", skip(self))]
    pub async fn run(&'static self) -> anyhow::Result<()> {
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

            self.execute_command(command, stream).await;
        }

        Ok(())
    }

    #[instrument(name = "exec", skip(self, stream))]
    async fn execute_command(&self, command: Command, stream: &mut TcpStream) {
        let result = match command {
            Command::Ping => stream.write(PONG_RESPONSE.as_bytes()).await,
            Command::Echo { message } => {
                stream
                    .write((format!("{SIMPLE_STRING_START}{message}{CRLF}")).as_bytes())
                    .await
            }
            Command::Set { key, value } => {
                self.db.lock().await.set(key, value);
                stream
                    .write((format!("{SIMPLE_STRING_START}OK{CRLF}")).as_bytes())
                    .await
            }
            Command::Get { key } => {
                let db = self.db.lock().await;
                let response: String = match db.get(&key) {
                    Ok(value) => format!("+{}", value.data),
                    Err(Error::KeyNotFound) => "-Key not found".to_string(),
                    Err(Error::Expired) => "$-1".to_string(),
                };
                stream.write(format!("{response}{CRLF}").as_bytes()).await
            }
        };

        match result {
            Ok(n) => tracing::trace!("Responded with {n} bytes"),
            Err(err) => tracing::error!("Error: {err}"),
        }
    }
}
