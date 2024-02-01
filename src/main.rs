use std::net::TcpListener;

fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379")?;

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => println!("accepted new connection"),
            Err(e) => println!("error: {}", e),
        }
    }

    Ok(())
}
