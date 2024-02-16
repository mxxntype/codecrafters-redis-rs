//! In this challenge, you'll build a toy Redis clone that's capable of handling
//! basic commands like `PING`, `SET` and `GET`. Along the way we'll learn about
//! event loops, the Redis protocol and more.
//!
//! **Note**: If you're viewing this repo on GitHub, head over to
//! [codecrafters.io](https://codecrafters.io) to try the challenge.

mod command;
mod database;
mod resp;
mod server;

use server::Server;
use tracing::Level;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup();

    let server = Server::new().await?;
    server.run().await?;

    Ok(())
}

fn setup() {
    let _ = color_eyre::install();
    fmt::Subscriber::builder()
        .with_max_level(Level::TRACE)
        .init();
    tracing::trace!("Setup hook finished");
}
