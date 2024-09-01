use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::sync::Arc;
use std::{
    env,
    io::{self},
};
use tokio::sync::Notify;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

use futures::FutureExt;
use std::io::Write;

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

    // Split the WebSocket stream into read and write halves
    let (mut write, mut read) = ws_stream.split();

    // Notify to signal when the client should shut down
    let shutdown_notify = Arc::new(Notify::new());
    let shutdown_notify_clone = Arc::clone(&shutdown_notify);

    // Task to read messages from the server and print them
    let read_task = tokio::spawn(async move {
        while let Some(Ok(message)) = read.next().await {
            match message {
                Message::Text(text) => {
                    let chat_msg: ChatMessage = serde_json::from_str(&text).unwrap();
                    println!("\n{}", chat_msg);
                }
                Message::Close(_) => {
                    println!("Connection closed by the server.");
                    shutdown_notify_clone.notify_one();
                    break;
                }
                _ => {}
            }
            print_prompt(); // Display prompt again after receiving a message
        }
    });

    // Task to handle user input and sending messages
    let input_task = tokio::spawn(async move {
        let stdin = io::stdin();
        loop {
            print_prompt();

            let mut input = String::new();
            stdin.read_line(&mut input).expect("Failed to read input");
            let input = input.trim();

            if input.to_lowercase() == "leave" {
                println!("Disconnecting from the server...");
                let _ = write.close().await;
                break;
            } else if let Some(stripped) = input.strip_prefix("send ") {
                let message = stripped.trim();
                if !message.is_empty() {
                    if let Err(e) = write.send(Message::Text(message.to_string())).await {
                        println!("Failed to send message to the server: {}", e);
                        break;
                    }
                }
            } else if input.is_empty() {
            } else {
                println!(
                    "Unknown command. Use 'send <message>' to send a message or 'leave' to disconnect."
                );
            }

            // Check if the shutdown signal has been received
            if shutdown_notify.notified().now_or_never().is_some() {
                println!("Shutting down client...");
                break;
            }
        }
    });

    // Wait for both tasks to complete
    let _ = tokio::join!(read_task, input_task);

    println!("Client has shut down.");
}

fn print_prompt() {
    print!("> ");
    io::stdout().flush().unwrap();
}
