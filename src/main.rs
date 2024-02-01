//! In this challenge, you'll build a toy Redis clone that's capable of handling
//! basic commands like `PING`, `SET` and `GET`. Along the way we'll learn about
//! event loops, the Redis protocol and more.
//!
//! **Note**: If you're viewing this repo on GitHub, head over to
//! [codecrafters.io](https://codecrafters.io) to try the challenge.

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

const PONG_RESPONSE: &[u8] = b"+PONG\r\n";

fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379")?;

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");
                handle_client(&mut stream)?;
            }
            Err(e) => println!("error accepting connection: {}", e),
        }
    }

    Ok(())
}

// FIXME: Handles a single request like "ping\nping\nping" fine,
// but any following requests hang?..
fn handle_client(stream: &mut TcpStream) -> anyhow::Result<()> {
    let mut buf = [0u8; 20];
    while let Ok(_read_bytes) = stream.read(&mut buf) {
        _ = stream.write(PONG_RESPONSE);
    }
    Ok(())
}
