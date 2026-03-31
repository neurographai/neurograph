// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Conversation history management.

use super::{Message, Role, SourceCitation};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Manages conversation history with persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistory {
    pub id: String,
    pub messages: Vec<Message>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl ConversationHistory {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_user_message(&mut self, content: &str) {
        self.messages.push(Message {
            role: Role::User,
            content: content.to_string(),
            timestamp: Utc::now(),
            sources: Vec::new(),
        });
        self.updated_at = Utc::now();
    }

    pub fn add_assistant_message(&mut self, content: &str, sources: Vec<SourceCitation>) {
        self.messages.push(Message {
            role: Role::Assistant,
            content: content.to_string(),
            timestamp: Utc::now(),
            sources,
        });
        self.updated_at = Utc::now();
    }

    pub fn last_n(&self, n: usize) -> &[Message] {
        let start = self.messages.len().saturating_sub(n);
        &self.messages[start..]
    }

    pub fn all(&self) -> &[Message] {
        &self.messages
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let history: Self = serde_json::from_str(&json)?;
        Ok(history)
    }
}

impl Default for ConversationHistory {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_flow() {
        let mut history = ConversationHistory::new();
        assert!(history.is_empty());
        history.add_user_message("What is attention?");
        history.add_assistant_message("Attention is a mechanism...", vec![]);
        assert_eq!(history.len(), 2);
        assert_eq!(history.messages[0].role, Role::User);
        assert_eq!(history.messages[1].role, Role::Assistant);
    }

    #[test]
    fn test_last_n() {
        let mut history = ConversationHistory::new();
        for i in 0..10 {
            history.add_user_message(&format!("Message {}", i));
        }
        let last3 = history.last_n(3);
        assert_eq!(last3.len(), 3);
    }
}
