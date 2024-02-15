//! In this challenge, you'll build a toy Redis clone that's capable of handling
//! basic commands like `PING`, `SET` and `GET`. Along the way we'll learn about
//! event loops, the Redis protocol and more.
//!
//! **Note**: If you're viewing this repo on GitHub, head over to
//! [codecrafters.io](https://codecrafters.io) to try the challenge.

mod command;
mod database;
mod resp;

use crate::{
    command::{Command, PONG_RESPONSE},
    database::{Database, Error},
    resp::{Token, CRLF, SIMPLE_STRING_START},
};
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use tracing::{instrument, Level};
use tracing_subscriber::fmt;

const LISTEN_ADDR: &str = "127.0.0.1:6379";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup();

    let listener = TcpListener::bind(LISTEN_ADDR).await?;
    let db = Arc::new(Mutex::new(Database::new()));

    loop {
        let (mut socket, addr) = listener.accept().await?;
        tracing::info!("Accepted a connection from {}", addr);
        let db = db.clone();
        tokio::spawn(async move {
            let _ = handle_client(&mut socket, db)
                .await
                .map_err(|err| tracing::error!("Error handling client: {err}"));
            tracing::info!("{} closed the connection", addr);
        });
    }
}

#[instrument(skip(stream, db))]
async fn handle_client(stream: &mut TcpStream, db: Arc<Mutex<Database>>) -> anyhow::Result<()> {
    let mut request = [0; 512];

    // `stream.read()` reads until a newline, so lets
    // run it in a loop to read everything line-by-line.
    while let Ok(read_bytes) = stream.read(&mut request).await {
        // Having nothing to read is not an error, it's an Ok(0).
        // Without this, the loop will run until an error occurs.
        if read_bytes == 0 {
            tracing::trace!("Stream read finished");
            break;
        }

        // If we actually read something meaningful, respond to it.
        let string = String::from_utf8(request.to_vec())?;
        let syntax = Token::try_from(string.as_str())?;
        let command = Command::try_from(syntax)?;
        tracing::debug!("Command = {command:?}");

        match command {
            Command::Ping => {
                let _ = stream.write(PONG_RESPONSE.as_bytes()).await.unwrap();
            }
            Command::Echo { message } => {
                let _ = stream
                    .write((format!("{SIMPLE_STRING_START}{message}{CRLF}")).as_bytes())
                    .await
                    .unwrap();
            }
            Command::Set { key, value } => {
                db.lock().await.set(key, value);
                let _ = stream
                    .write((format!("{SIMPLE_STRING_START}OK{CRLF}")).as_bytes())
                    .await
                    .unwrap();
            }
            Command::Get { key } => {
                let db = db.lock().await;
                let response: String = match db.get(&key) {
                    Ok(value) => format!("+{}", value.data),
                    Err(Error::KeyNotFound) => "-Key not found".to_string(),
                    Err(Error::Expired) => "$-1".to_string(),
                };
                let _ = stream
                    .write(format!("{response}{CRLF}").as_bytes())
                    .await
                    .unwrap();
            }
        }
    }

    Ok(())
}

fn setup() {
    let _ = color_eyre::install();
    fmt::Subscriber::builder()
        .with_max_level(Level::TRACE)
        .init();
    tracing::trace!("Setup hook finished");
}
