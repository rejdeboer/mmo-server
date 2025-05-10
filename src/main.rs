use std::error::Error;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const SERVER_ADDR: &str = "127.0.0.1:8000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(SERVER_ADDR).await?;
    println!("[Server] Listening on: {}", SERVER_ADDR);

    loop {
        // `accept()` returns a tuple of `(TcpStream, SocketAddr)`
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("[Server] Accepted new connection from: {}", addr);

                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, addr).await {
                        eprintln!("[Server] Error handling client {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                eprintln!("[Server] Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_client(mut stream: TcpStream, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    println!("[Client {}] Connected.", addr);

    let mut buffer = [0u8; 1024];

    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => {
                println!("[Client {}] Connection closed by client.", addr);
                return Ok(());
            }
            Ok(n) => {
                let received_data = &buffer[..n];
                let received_string = String::from_utf8_lossy(received_data);

                println!("[Client {}] Received: {}", addr, received_string.trim_end());

                if let Err(e) = stream.write_all(received_data).await {
                    eprintln!("[Client {}] Failed to write to stream: {}", addr, e);
                    return Err(e.into());
                }
                println!(
                    "[Client {}] Echoed back: {}",
                    addr,
                    received_string.trim_end()
                );
            }
            Err(e) => {
                eprintln!("[Client {}] Failed to read from stream: {}", addr, e);
                return Err(e.into());
            }
        }
    }
}
