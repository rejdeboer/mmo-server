// This is a conceptual Rust snippet, not a full MMO server!
// You'd use Tokio or async-std for a real server.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle_client(mut stream: TcpStream) {
    let peer_addr = stream
        .peer_addr()
        .unwrap_or_else(|_| "unknown".parse().unwrap());
    println!("New client connected: {}", peer_addr);
    let mut buffer = [0; 1024]; // Buffer for reading

    loop {
        match stream.read(&mut buffer) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    // Connection closed by client
                    println!("Client {} disconnected.", peer_addr);
                    break;
                }
                let received_str = String::from_utf8_lossy(&buffer[..bytes_read]);
                println!("Received from {}: {}", peer_addr, received_str);

                // Echo back
                let response = format!("Server received: {}", received_str);
                if let Err(e) = stream.write_all(response.as_bytes()) {
                    eprintln!("Failed to send response to {}: {}", peer_addr, e);
                    break;
                }
                println!("Sent to {}: {}", peer_addr, response);
            }
            Err(e) => {
                eprintln!("Error reading from client {}: {}", peer_addr, e);
                break;
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8000")?;
    println!("Server listening on 127.0.0.1:8000");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    handle_client(stream);
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }
    Ok(())
}
