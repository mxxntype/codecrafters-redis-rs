use std::{io::Write, net::TcpListener};

fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379")?;

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");
                stream.write(b"+PONG\r\n")?;
            }
            Err(e) => println!("error: {}", e),
        }
    }

    Ok(())
}
