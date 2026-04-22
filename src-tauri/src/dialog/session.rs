// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Session management for dialog engine
//!
//! Tracks conversation state across multiple message exchanges

use crate::database::Database;
use crate::database::repositories::SessionRepository;
use crate::error::Result;
use crate::dialog::intent::Intent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Dialog session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogState {
    /// Waiting for user input
    Idle,
    /// Processing a command
    Processing,
    /// Waiting for slot fill
    WaitingForSlot,
    /// Waiting for confirmation
    WaitingForConfirmation,
    /// Error state
    Error,
    /// Session ended
    Ended,
}

impl DialogState {
    pub fn as_str(&self) -> &'static str {
        match self {
            DialogState::Idle => "idle",
            DialogState::Processing => "processing",
            DialogState::WaitingForSlot => "waiting_for_slot",
            DialogState::WaitingForConfirmation => "waiting_for_confirmation",
            DialogState::Error => "error",
            DialogState::Ended => "ended",
        }
    }
}

/// Dialog session tracking a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogSession {
    pub id: String,
    pub user_id: String,
    pub chat_id: String,
    pub platform: String,
    pub state: DialogState,
    pub current_intent: Option<Intent>,
    pub message_history: Vec<DialogMessage>,
    pub context: HashMap<String, serde_json::Value>,
    pub created_at: i64,
    pub last_active: i64,
    pub turn_count: u32,
}

/// A single message in the dialog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: i64,
    pub intent: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Session manager
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, DialogSession>>>,
    db: Option<Arc<Database>>,
    max_history: usize,
    session_timeout_secs: i64,
}

impl SessionManager {
    pub fn new(db: Option<Arc<Database>>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            db,
            max_history: 100,
            session_timeout_secs: 3600, // 1 hour
        }
    }

    /// Get or create a session for a user/chat combination
    pub async fn get_or_create(
        &self,
        user_id: &str,
        chat_id: &str,
        platform: &str,
    ) -> Result<DialogSession> {
        let session_key = format!("{}:{}:{}", platform, chat_id, user_id);

        let mut sessions = self.sessions.write().await;

        // Check for existing session
        if let Some(session) = sessions.get(&session_key) {
            // Check if session has timed out
            let now = chrono::Utc::now().timestamp();
            if now - session.last_active > self.session_timeout_secs {
                info!("Session {} timed out, creating new", session_key);
                sessions.remove(&session_key);
            } else {
                return Ok(session.clone());
            }
        }

        // Create new session
        let now = chrono::Utc::now().timestamp();
        let session = DialogSession {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            chat_id: chat_id.to_string(),
            platform: platform.to_string(),
            state: DialogState::Idle,
            current_intent: None,
            message_history: Vec::new(),
            context: HashMap::new(),
            created_at: now,
            last_active: now,
            turn_count: 0,
        };

        sessions.insert(session_key, session.clone());

        info!("Created new session {} for user {} on {}", session.id, user_id, platform);
        Ok(session)
    }

    /// Update a session
    pub async fn update_session(&self, session: DialogSession) -> Result<()> {
        let session_key = format!("{}:{}:{}", session.platform, session.chat_id, session.user_id);

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_key, session);

        Ok(())
    }

    /// Add a user message to session history
    pub async fn add_user_message(
        &self,
        session: &mut DialogSession,
        content: &str,
        intent_name: Option<&str>,
    ) -> Result<()> {
        let msg = DialogMessage {
            role: MessageRole::User,
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            intent: intent_name.map(|s| s.to_string()),
            metadata: HashMap::new(),
        };

        session.message_history.push(msg);
        session.last_active = chrono::Utc::now().timestamp();
        session.turn_count += 1;

        // Trim history if too long
        if session.message_history.len() > self.max_history {
            let excess = session.message_history.len() - self.max_history;
            session.message_history.drain(0..excess);
        }

        self.update_session(session.clone()).await
    }

    /// Add an assistant message to session history
    pub async fn add_assistant_message(
        &self,
        session: &mut DialogSession,
        content: &str,
    ) -> Result<()> {
        let msg = DialogMessage {
            role: MessageRole::Assistant,
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            intent: None,
            metadata: HashMap::new(),
        };

        session.message_history.push(msg);
        session.last_active = chrono::Utc::now().timestamp();

        self.update_session(session.clone()).await
    }

    /// Set the current intent for a session
    pub async fn set_intent(&self, session: &mut DialogSession, intent: Intent) -> Result<()> {
        session.current_intent = Some(intent);
        session.state = DialogState::Processing;
        self.update_session(session.clone()).await
    }

    /// Clear the current intent
    pub async fn clear_intent(&self, session: &mut DialogSession) -> Result<()> {
        session.current_intent = None;
        session.state = DialogState::Idle;
        self.update_session(session.clone()).await
    }

    /// Set session state
    pub async fn set_state(&self, session: &mut DialogSession, state: DialogState) -> Result<()> {
        session.state = state;
        self.update_session(session.clone()).await
    }

    /// Set context value
    pub async fn set_context(
        &self,
        session: &mut DialogSession,
        key: &str,
        value: serde_json::Value,
    ) -> Result<()> {
        session.context.insert(key.to_string(), value);
        self.update_session(session.clone()).await
    }

    /// Get context value
    pub fn get_context(&self, session: &DialogSession, key: &str) -> Option<&serde_json::Value> {
        session.context.get(key)
    }

    /// End a session
    pub async fn end_session(&self, session: &mut DialogSession) -> Result<()> {
        session.state = DialogState::Ended;
        let session_key = format!("{}:{}:{}", session.platform, session.chat_id, session.user_id);

        let mut sessions = self.sessions.write().await;
        sessions.remove(&session_key);

        info!("Session {} ended after {} turns", session.id, session.turn_count);
        Ok(())
    }

    /// Cleanup expired sessions
    pub async fn cleanup_expired(&self) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        let mut sessions = self.sessions.write().await;

        let expired_keys: Vec<String> = sessions.iter()
            .filter(|(_, session)| now - session.last_active > self.session_timeout_secs)
            .map(|(key, _)| key.clone())
            .collect();

        let count = expired_keys.len();
        for key in expired_keys {
            sessions.remove(&key);
        }

        if count > 0 {
            info!("Cleaned up {} expired sessions", count);
        }

        Ok(count)
    }

    /// Get active session count
    pub async fn active_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Get recent message history
    pub fn get_recent_history(&self, session: &DialogSession, count: usize) -> Vec<&DialogMessage> {
        session.message_history.iter().rev().take(count).collect()
    }

    /// Get conversation summary
    pub fn get_summary(&self, session: &DialogSession) -> String {
        let user_msgs = session.message_history.iter()
            .filter(|m| m.role == MessageRole::User)
            .count();
        let assistant_msgs = session.message_history.iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .count();

        format!(
            "会话 {} ({} 轮对话, {} 条用户消息, {} 条助手消息, 状态: {})",
            &session.id[..8],
            session.turn_count,
            user_msgs,
            assistant_msgs,
            session.state.as_str(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_create() {
        let manager = SessionManager::new(None);
        let session = manager.get_or_create("user1", "chat1", "feishu").await.unwrap();
        assert_eq!(session.user_id, "user1");
        assert_eq!(session.state, DialogState::Idle);
    }

    #[tokio::test]
    async fn test_session_add_message() {
        let manager = SessionManager::new(None);
        let mut session = manager.get_or_create("user1", "chat1", "feishu").await.unwrap();

        manager.add_user_message(&mut session, "你好", Some("greeting")).await.unwrap();
        assert_eq!(session.message_history.len(), 1);
        assert_eq!(session.turn_count, 1);
    }

    #[tokio::test]
    async fn test_session_cleanup() {
        let manager = SessionManager::new(None);
        let _ = manager.get_or_create("user1", "chat1", "feishu").await.unwrap();

        let cleaned = manager.cleanup_expired().await.unwrap();
        assert_eq!(cleaned, 0); // Not expired yet
    }
}
