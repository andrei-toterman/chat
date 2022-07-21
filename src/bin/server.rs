use std::sync::Arc;

use tokio::net::TcpListener;

use chat::server::{handle_client, State};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let address = std::env::args().nth(1).expect("Usage: server <address>");
    let listener = TcpListener::bind(address).await?;
    let state = Arc::new(State::default());

    loop {
        let (stream, address) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            match handle_client(state, stream, address).await {
                Ok(_) => println!("client {address} terminated successfully"),
                Err(err) => eprintln!("client {address} failed: {err:?}"),
            }
        });
    }
}
