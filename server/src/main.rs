use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use types::ChatMessage;

struct AppState {
    clients: Mutex<HashSet<String>>,
    sender: broadcast::Sender<ChatMessage>,
}

// WebSocket handler that upgrades the connection and handles incoming and outgoing messages
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

// Handle a WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let mut bc_receiver = state.sender.subscribe(); // Subscribe to broadcast channel

    let mut username = String::new();

    // Receive the first message, which should contain the username
    if let Some(Ok(Message::Text(name))) = ws_receiver.next().await {
        username = name.trim().to_string();

        // Check if the username is already taken
        let mut clients = state.clients.lock().await;

        if clients.contains(&username) {
            // If username is taken, send an error message and close the connection
            let error_message = ChatMessage::new(
                "Host",
                &format!(
                    "Username '{}' is already taken. Please choose a different one.",
                    username
                ),
            );
            let _ = ws_sender
                .send(Message::Text(
                    serde_json::to_string(&error_message).unwrap(),
                ))
                .await;

            // Send a close signal to the client
            let _ = ws_sender.send(Message::Close(None)).await;

            println!("Rejected user '{}' due to duplicate username.", username);

            return; // Exit the function to terminate the connection
        }

        // If username is unique, add it to the set of clients
        clients.insert(username.clone());
        println!("User '{}' joined the chat", username);

        // Notify all users about the new user
        let welcome_message =
            ChatMessage::new("Host", &format!("{} has joined the chat!", username));

        // Broadcast the join message
        state.sender.send(welcome_message.clone()).unwrap();
    }

    // Main loop to handle incoming and outgoing messages

    loop {
        tokio::select! {

            // Handle incoming WebSocket messages
         result = ws_receiver.next() => {
                match result {
                    Some(Ok(msg)) => {
                        if let Message::Text(text) = msg {
                            let chat_msg = ChatMessage::new(
                                &username,
                                &text
                            );

                            // Broadcast the message to all clients
                            if state.sender.send(chat_msg.clone()).is_err() {
                                println!("No active subscribers to receive the message");
                            }
                        }
                    }

                    Some(Err(_)) => {
                        println!("Error while receiving a message from '{}'.", username);
                        break; // Exit the loop on error
                    }

                    None => {
                        println!("Client '{}' disconnected.", username);
                        break; // Exit the loop when the client disconnects (None)
                    }
                }
         }

            // Handle outgoing WebSocket messages
            Ok(chat_msg) = bc_receiver.recv() => {
                if chat_msg.get_username() != username {
                    let msg_text = serde_json::to_string(&chat_msg).unwrap();
                    if ws_sender.send(Message::Text(msg_text)).await.is_err() {
                        break; // If sending fails, break the loop to close the connection
                    }
                }
            }
        }
    }

    // Cleanup: Remove the client from the state
    let mut clients = state.clients.lock().await;
    clients.remove(&username);
    println!("User '{}' left the chat", username);
}

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel(100);

    let state = Arc::new(AppState {
        clients: Mutex::new(HashSet::new()),
        sender: tx,
    });

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(state);

    let addr = "127.0.0.1:3000";
    println!("Chat server running on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
