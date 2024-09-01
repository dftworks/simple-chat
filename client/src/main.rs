use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::{
    env,
    io::{self},
};

use std::io::Write;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

use types::ChatMessage;

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
    let (mut ws_stream, _) = connect_async(url.to_string())
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
                Message::Text(text) => {
                    let chat_msg: ChatMessage = serde_json::from_str(&text).unwrap();
                    println!("\n{}", chat_msg);
                }

                Message::Close(_) => {
                    println!("Connection closed by the server.");
                    break;
                }
                _ => {}
            }
            print_prompt(); // Display prompt again after receiving a message
        }
    });

    // Interactive command prompt for user input
    let stdin = io::stdin();
    loop {
        print_prompt();

        let mut input = String::new();
        stdin.read_line(&mut input).expect("Failed to read input");
        let input = input.trim();

        if input.to_lowercase() == "leave" {
            println!("Disconnecting from the server...");
            write.close().await.expect("Failed to close connection");
            break;
        } else if input.starts_with("send ") {
            let message = input[5..].trim();
            if !message.is_empty() {
                write
                    .send(Message::Text(message.to_string()))
                    .await
                    .expect("Failed to send message to the server");
            }
        } else {
            println!(
                "Unknown command. Use 'send <message>' to send a message or 'leave' to disconnect."
            );
        }
    }
}

fn print_prompt() {
    print!("> ");
    io::stdout().flush().unwrap();
}
