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
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChatMessage {
    username: String,
    content: String,
}

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
    let mut username = String::new();
    let mut bc_receiver = state.sender.subscribe(); // Subscribe to broadcast channel

    // Receive the first message, which should contain the username
    if let Some(Ok(msg)) = ws_receiver.next().await {
        if let Message::Text(name) = msg {
            username = name.trim().to_string();
            println!("User '{}' joined the chat", username);

            // Notify all users about the new user
            let welcome_message = ChatMessage {
                username: "Host".to_string(),
                content: format!("{} has joined the chat!", username),
            };

            // Broadcast the join message
            state.sender.send(welcome_message.clone()).unwrap();

            // Store client sender to handle disconnects (if needed)
            let mut clients = state.clients.lock().unwrap();
            clients.insert(username.clone());
        }
    }

    // Main loop to handle incoming and outgoing messages
    loop {
        tokio::select! {
            // Handle incoming WebSocket messages
         result = ws_receiver.next() => {
                match result {
                    Some(Ok(msg)) => {
                        if let Message::Text(text) = msg {
                            let chat_msg = ChatMessage {
                                username: username.clone(),
                                content: text.clone(),
                            };

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
                if chat_msg.username != username {
                    let msg_text = serde_json::to_string(&chat_msg).unwrap();
                    if ws_sender.send(Message::Text(msg_text)).await.is_err() {
                        break; // If sending fails, break the loop to close the connection
                    }
                }
            }
        }
    }

    // Cleanup: Remove the client from the state
    let mut clients = state.clients.lock().unwrap();
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
