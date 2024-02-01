use std::{io::Write, net::TcpListener};

const PONG_RESPONSE: &[u8] = b"+PONG\r\n";

fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379")?;

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");
                let _ = stream.write(PONG_RESPONSE)?;
            }
            Err(e) => println!("error accepting connection: {}", e),
        }
    }

    Ok(())
}
