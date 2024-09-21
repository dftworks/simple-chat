// server/tests/server_test.rs

use tokio::process::Command; // Use tokio's Command, not std
use tokio::time::{sleep, Duration};
use tokio::net::TcpStream;
use tokio::process::Child;

async fn start_server() -> Child {
    // Spawn the server using tokio's async Command
    let server_process = Command::new("cargo")
        .args(&["run", "--bin", "server"])
        .current_dir("../")  // Ensure that it runs from the workspace root
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start server");

    // Wait for the server to start
    sleep(Duration::from_secs(2)).await;

    server_process
}

async fn check_server_connection(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).await.is_ok()
}

#[tokio::test]
async fn test_server_starts() {
    // Start the server
    let mut server = start_server().await;

    // Check if the server is accepting connections
    assert!(check_server_connection(3000).await, "Server failed to start");

    // Kill the server after the test
    server.kill().await.expect("Failed to stop the server");
}

