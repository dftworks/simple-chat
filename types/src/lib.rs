use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    username: String,
    content: String,
}

impl ChatMessage {
    pub fn new(username: &str, content: &str) -> ChatMessage {
        ChatMessage {
            username: username.to_string(),
            content: content.to_string(),
        }
    }

    pub fn get_username(&self) -> String {
        self.username.clone()
    }

    pub fn get_content(&self) -> String {
        self.content.clone()
    }
}

impl fmt::Display for ChatMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.username, self.content)
    }
}
