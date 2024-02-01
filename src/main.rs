//! In this challenge, you'll build a toy Redis clone that's capable of handling
//! basic commands like `PING`, `SET` and `GET`. Along the way we'll learn about
//! event loops, the Redis protocol and more.
//!
//! **Note**: If you're viewing this repo on GitHub, head over to
//! [codecrafters.io](https://codecrafters.io) to try the challenge.

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const PONG_RESPONSE: &[u8] = b"+PONG\r\n";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    let mut threads = vec![];
    loop {
        let (mut socket, _) = listener.accept().await?;
        let thread = tokio::spawn(async move { handle_client(&mut socket).await.unwrap() });
        threads.push(thread);
    }
}

async fn handle_client(stream: &mut TcpStream) -> anyhow::Result<()> {
    let mut request = [0u8; 32];

    // `stream.read()` reads until a newline, so lets
    // run it in a loop to read everything line-by-line.
    while let Ok(read_bytes) = stream.read(&mut request).await {
        // Having nothing to read is not an error, it's an Ok(0).
        // Without this, the loop will run until an error occurs.
        if read_bytes == 0 {
            break;
        }

        // If we actually read something meaningful, respond to it.
        _ = stream.write(PONG_RESPONSE).await?;
    }

    Ok(())
}
