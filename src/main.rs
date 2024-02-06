//! In this challenge, you'll build a toy Redis clone that's capable of handling
//! basic commands like `PING`, `SET` and `GET`. Along the way we'll learn about
//! event loops, the Redis protocol and more.
//!
//! **Note**: If you're viewing this repo on GitHub, head over to
//! [codecrafters.io](https://codecrafters.io) to try the challenge.

mod command;
mod resp;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use command::{Command, PONG_RESPONSE};
use resp::{SEPARATOR, SIMPLE_STRING_START};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const LISTEN_ADDR: &str = "127.0.0.1:6379";

type Database = HashMap<String, Value>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Value {
    pub(crate) value: String,
    pub(crate) ttl: Option<Duration>,
    pub(crate) created: Instant,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind(LISTEN_ADDR).await?;
    let db = Arc::new(Mutex::new(Database::new()));

    // let mut threads = vec![];
    loop {
        let (mut socket, _) = listener.accept().await?;
        let db = db.clone();
        tokio::spawn(async move {
            handle_client(&mut socket, db).await.unwrap();
        });
        // threads.push(handle);
    }
}

async fn handle_client(stream: &mut TcpStream, db: Arc<Mutex<Database>>) -> anyhow::Result<()> {
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
        let syntax = String::from_utf8(request.to_vec())?;
        let command = Command::try_from(syntax.as_str())?;

        match command {
            Command::Ping => {
                let _ = stream.write(PONG_RESPONSE.as_bytes()).await.unwrap();
            }
            Command::Echo { message } => {
                let _ = stream
                    .write((format!("{SIMPLE_STRING_START}{message}{SEPARATOR}")).as_bytes())
                    .await
                    .unwrap();
            }
            Command::Set { key, value } => {
                db.lock().unwrap().insert(key, value);
                let _ = stream
                    .write((format!("{SIMPLE_STRING_START}OK{SEPARATOR}")).as_bytes())
                    .await
                    .unwrap();
            }
            Command::Get { key } => {
                let requested = Instant::now();
                let _ = stream
                    .write(
                        format!(
                            "{SIMPLE_STRING_START}{}{SEPARATOR}",
                            match db.lock().unwrap().get(&key) {
                                Some(value) => {
                                    match value.ttl {
                                        Some(ttl) => {
                                            if requested - value.created <= ttl {
                                                value.value.as_str()
                                            } else {
                                                // let _ = db.lock().unwrap().remove(&key);
                                                "ERR: expired"
                                            }
                                        }
                                        None => value.value.as_str(),
                                    }
                                }
                                None => "ERR: no such key",
                            }
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
            }
        }
    }

    Ok(())
}
