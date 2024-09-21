use tokio::process::Command; // Use tokio's Command, not std
use tokio::time::{sleep, Duration};
use tokio::net::TcpStream;
use tokio::process::Child;
use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncBufReadExt, BufReader};

async fn start_client(address: &str, port: u16, username: &str) -> Child {
    // Start the client using tokio's async Command
    let client_process = Command::new("cargo")
        .args(&[
            "run", 
            "--bin", "client", 
            address, 
            &port.to_string(), 
            username
        ])
        .current_dir("../")  // Ensure that it runs from the workspace root
        .stdout(std::process::Stdio::piped())
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start client");

    sleep(Duration::from_secs(1)).await; // Give the client some time to connect

    client_process
}

// Function to check if the server is up and accepting connections
async fn check_server_connection(port: u16) -> bool {
    for _ in 0..20 {  // Retry for 10 seconds (20 attempts with 500ms delay)
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return true;
        }
        sleep(Duration::from_millis(500)).await; // Wait 500 ms between retries
    }
    false
}

// Function to print the server output (stdout and stderr)
async fn print_server_output(server_stdout: tokio::process::ChildStdout, server_stderr: tokio::process::ChildStderr) {
    let mut stdout_reader = BufReader::new(server_stdout).lines();
    let mut stderr_reader = BufReader::new(server_stderr).lines();

    // Create a tokio task to print server stdout
    tokio::spawn(async move {
        while let Some(line) = stdout_reader.next_line().await.unwrap_or(None) {
            println!("Server stdout: {}", line);
        }
    });

    // Create a tokio task to print server stderr
    tokio::spawn(async move {
        while let Some(line) = stderr_reader.next_line().await.unwrap_or(None) {
            eprintln!("Server stderr: {}", line);
        }
    });
}

#[tokio::test]
async fn test_client_server_interaction() {
    // Start the server
    let mut server = Command::new("cargo")
        .args(&["run", "--bin", "server"])
        .current_dir("../")  // Ensure that it runs from the workspace root
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start server");

    // Capture and print server output
    let server_stdout = server.stdout.take().expect("Failed to capture server stdout");
    let server_stderr = server.stderr.take().expect("Failed to capture server stderr");
    print_server_output(server_stdout, server_stderr).await;

    // Ensure the server is up before starting the client
    assert!(
        check_server_connection(3000).await,
        "Server is not accepting connections"
    );

    // Start the client
    let mut client = start_client("127.0.0.1", 3000, "testuser").await;

    // Simulate sending a message from the client
    let client_stdin = client.stdin.as_mut().expect("Failed to open client stdin");
    let message = "send Hello, server!\n";
    client_stdin.write_all(message.as_bytes()).await.expect("Failed to send message");

    sleep(Duration::from_secs(2)).await; // Give the client time to send the message

    // Kill the client
    client.kill().await.expect("Failed to stop client");

    // Kill the server after the test
    server.kill().await.expect("Failed to stop the server");
}

