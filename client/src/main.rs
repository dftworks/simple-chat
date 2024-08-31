use futures_util::{SinkExt, StreamExt};
use std::{
    env,
    io::{self, BufRead},
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use url::Url;

#[tokio::main]
async fn main() {
    // Read command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <address> <port> <username>", args[0]);
        return;
    }

    let address = &args[1];
    let port = &args[2];
    let username = &args[3];

    // Construct the WebSocket URL
    let server_url = format!("ws://{}:{}/ws", address, port);
    let url = Url::parse(&server_url).expect("Invalid WebSocket URL");

    // Connect to the WebSocket server
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to the WebSocket server");

    println!(
        "Connected to the WebSocket server at {}:{} as '{}'",
        address, port, username
    );

    // Send the username to the server as the first message
    ws_stream
        .send(Message::Text(username.clone()))
        .await
        .expect("Failed to send username");

    // Clone the WebSocket stream for reading and writing
    let (mut write, mut read) = ws_stream.split();

    // Task to read messages from the server and print them
    tokio::spawn(async move {
        while let Some(Ok(message)) = read.next().await {
            match message {
                Message::Text(text) => println!("Received: {}", text),
                Message::Close(_) => {
                    println!("Connection closed by the server.");
                    break;
                }
                _ => {}
            }
        }
    });

    // Read user input from the console and send it to the server
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.expect("Failed to read line from stdin");
        write
            .send(Message::Text(line))
            .await
            .expect("Failed to send message to the server");
    }
}
