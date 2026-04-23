// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Main dialog engine
//!
//! Orchestrates intent recognition, session management, and command execution

#![allow(dead_code)]
use crate::database::Database;
use crate::dialog::intent::{IntentRecognizer, Intent, SlotValue};
use crate::dialog::session::{SessionManager, DialogSession, DialogState};
use crate::dialog::command::{
    CommandRegistry, CommandContext,
    HelpCommand, GreetingCommand, UnknownCommand,
};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, error};

/// Dialog engine configuration
#[derive(Debug, Clone)]
pub struct DialogEngineConfig {
    pub session_timeout_secs: i64,
    pub max_history: usize,
    pub confidence_threshold: f64,
}

impl Default for DialogEngineConfig {
    fn default() -> Self {
        Self {
            session_timeout_secs: 3600,
            max_history: 100,
            confidence_threshold: 0.3,
        }
    }
}

/// Dialog response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogResponse {
    pub message: String,
    pub intent_name: Option<String>,
    pub confidence: Option<f64>,
    pub state: String,
    pub data: Option<serde_json::Value>,
    pub follow_up: Option<crate::dialog::command::FollowUp>,
}

/// Main dialog engine
pub struct DialogEngine {
    intent_recognizer: Arc<IntentRecognizer>,
    session_manager: Arc<SessionManager>,
    command_registry: Arc<tokio::sync::RwLock<CommandRegistry>>,
    config: DialogEngineConfig,
}

impl DialogEngine {
    /// Create a new dialog engine
    pub fn new(db: Option<Arc<Database>>) -> Self {
        let config = DialogEngineConfig::default();

        let mut command_registry = CommandRegistry::new();
        command_registry.register(Arc::new(HelpCommand));
        command_registry.register(Arc::new(GreetingCommand));
        command_registry.register(Arc::new(UnknownCommand));

        Self {
            intent_recognizer: Arc::new(IntentRecognizer::new()),
            session_manager: Arc::new(SessionManager::new(db)),
            command_registry: Arc::new(tokio::sync::RwLock::new(command_registry)),
            config,
        }
    }

    /// Process an incoming message
    pub async fn process_message(
        &self,
        user_id: &str,
        chat_id: &str,
        platform: &str,
        message: &str,
    ) -> Result<DialogResponse> {
        info!("Processing message from {} on {}: {}", user_id, platform, message);

        // Get or create session
        let mut session = self.session_manager
            .get_or_create(user_id, chat_id, platform)
            .await?;

        // Add user message to history
        self.session_manager
            .add_user_message(&mut session, message, None)
            .await?;

        // Check if we're in the middle of a multi-turn dialog
        if session.state == DialogState::WaitingForSlot {
            return self.handle_slot_fill(&mut session, message).await;
        }

        if session.state == DialogState::WaitingForConfirmation {
            return self.handle_confirmation(&mut session, message).await;
        }

        // Recognize intent
        let intent = self.intent_recognizer.recognize(message)?;

        match intent {
            Some(intent) if intent.confidence >= self.config.confidence_threshold => {
                // Add intent to message history
                if let Some(last_msg) = session.message_history.last_mut() {
                    last_msg.intent = Some(intent.name.clone());
                }
                self.session_manager.update_session(session.clone()).await?;

                self.handle_intent(&mut session, intent).await
            }
            Some(intent) => {
                // Low confidence - ask for clarification
                warn!("Low confidence intent: {} ({:.2})", intent.name, intent.confidence);

                let response = DialogResponse {
                    message: format!(
                        "你是指「{}」吗？请确认或重新描述。",
                        self.describe_intent(&intent)
                    ),
                    intent_name: Some(intent.name.clone()),
                    confidence: Some(intent.confidence),
                    state: "clarification".to_string(),
                    data: None,
                    follow_up: None,
                };

                // Set session to confirmation state
                self.session_manager.set_intent(&mut session, intent).await?;
                self.session_manager.set_state(&mut session, DialogState::WaitingForConfirmation).await?;

                self.session_manager
                    .add_assistant_message(&mut session, &response.message)
                    .await?;

                Ok(response)
            }
            None => {
                // No intent recognized - fallback
                self.handle_fallback(&mut session).await
            }
        }
    }

    /// Handle a recognized intent
    async fn handle_intent(
        &self,
        session: &mut DialogSession,
        intent: Intent,
    ) -> Result<DialogResponse> {
        // Check for missing required slots
        let missing = self.intent_recognizer.get_missing_slots(&intent);

        if !missing.is_empty() {
            // Need more information
            let slot = missing.first().unwrap();
            self.session_manager.set_intent(session, intent.clone()).await?;
            self.session_manager.set_state(session, DialogState::WaitingForSlot).await?;

            let response = DialogResponse {
                message: slot.prompt.clone(),
                intent_name: Some(intent.name.clone()),
                confidence: Some(intent.confidence),
                state: "waiting_for_input".to_string(),
                data: None,
                follow_up: Some(crate::dialog::command::FollowUp {
                    follow_up_type: crate::dialog::command::FollowUpType::SlotFill {
                        slot_name: slot.name.clone(),
                    },
                    prompt: slot.prompt.clone(),
                    options: None,
                }),
            };

            self.session_manager
                .add_assistant_message(session, &response.message)
                .await?;

            return Ok(response);
        }

        // All slots filled - execute command
        self.session_manager.set_intent(session, intent.clone()).await?;
        self.session_manager.set_state(session, DialogState::Processing).await?;

        let context = CommandContext {
            user_id: session.user_id.clone(),
            chat_id: session.chat_id.clone(),
            platform: session.platform.clone(),
            session_id: session.id.clone(),
            intent: intent.clone(),
            extra: HashMap::new(),
        };

        let registry = self.command_registry.read().await;
        let result = if registry.has_command(&intent.name) {
            registry.execute(&intent.name, context).await
        } else {
            // Use unknown command fallback
            registry.execute("unknown", context).await
        };
        drop(registry);

        match result {
            Ok(cmd_result) => {
                self.session_manager.set_state(session, DialogState::Idle).await?;
                self.session_manager.clear_intent(session).await?;

                let response = DialogResponse {
                    message: cmd_result.message.clone(),
                    intent_name: Some(intent.name.clone()),
                    confidence: Some(intent.confidence),
                    state: "completed".to_string(),
                    data: cmd_result.data,
                    follow_up: cmd_result.follow_up,
                };

                self.session_manager
                    .add_assistant_message(session, &response.message)
                    .await?;

                Ok(response)
            }
            Err(e) => {
                error!("Command execution failed: {}", e);
                self.session_manager.set_state(session, DialogState::Error).await?;

                let response = DialogResponse {
                    message: format!("执行时出错：{}", e),
                    intent_name: Some(intent.name),
                    confidence: None,
                    state: "error".to_string(),
                    data: None,
                    follow_up: None,
                };

                self.session_manager
                    .add_assistant_message(session, &response.message)
                    .await?;

                self.session_manager.set_state(session, DialogState::Idle).await?;
                self.session_manager.clear_intent(session).await?;

                Ok(response)
            }
        }
    }

    /// Handle slot filling
    async fn handle_slot_fill(
        &self,
        session: &mut DialogSession,
        message: &str,
    ) -> Result<DialogResponse> {
        if let Some(ref mut intent) = session.current_intent {
            let missing = self.intent_recognizer.get_missing_slots(intent);

            if let Some(slot) = missing.first() {
                let slot_name = slot.name.clone();
                let value = SlotValue::Text(message.to_string());

                self.intent_recognizer.fill_slot(intent, &slot_name, value);
            }

            // Re-check for missing slots
            let still_missing = self.intent_recognizer.get_missing_slots(intent);

            if still_missing.is_empty() {
                // All slots filled - execute
                let intent = intent.clone();
                return self.handle_intent(session, intent).await;
            } else {
                // Still need more info
                let next_slot = still_missing.first().unwrap();
                let response = DialogResponse {
                    message: next_slot.prompt.clone(),
                    intent_name: Some(intent.name.clone()),
                    confidence: None,
                    state: "waiting_for_input".to_string(),
                    data: None,
                    follow_up: Some(crate::dialog::command::FollowUp {
                        follow_up_type: crate::dialog::command::FollowUpType::SlotFill {
                            slot_name: next_slot.name.clone(),
                        },
                        prompt: next_slot.prompt.clone(),
                        options: None,
                    }),
                };

                self.session_manager.update_session(session.clone()).await?;
                self.session_manager
                    .add_assistant_message(session, &response.message)
                    .await?;

                return Ok(response);
            }
        }

        // No current intent - reset
        self.session_manager.set_state(session, DialogState::Idle).await?;
        self.handle_fallback(session).await
    }

    /// Handle confirmation
    async fn handle_confirmation(
        &self,
        session: &mut DialogSession,
        message: &str,
    ) -> Result<DialogResponse> {
        let lower = message.to_lowercase();
        let confirmed = lower.contains("是") || lower.contains("yes")
            || lower.contains("确认") || lower.contains("对")
            || lower.contains("正确");

        if confirmed {
            if let Some(ref intent) = session.current_intent {
                let intent = intent.clone();
                return self.handle_intent(session, intent).await;
            }
        }

        // Denied - reset
        self.session_manager.set_state(session, DialogState::Idle).await?;
        self.session_manager.clear_intent(session).await?;

        let response = DialogResponse {
            message: "好的，请重新描述你的需求。".to_string(),
            intent_name: None,
            confidence: None,
            state: "idle".to_string(),
            data: None,
            follow_up: None,
        };

        self.session_manager
            .add_assistant_message(session, &response.message)
            .await?;

        Ok(response)
    }

    /// Handle unrecognized input
    async fn handle_fallback(
        &self,
        session: &mut DialogSession,
    ) -> Result<DialogResponse> {
        let registry = self.command_registry.read().await;
        let context = CommandContext {
            user_id: session.user_id.clone(),
            chat_id: session.chat_id.clone(),
            platform: session.platform.clone(),
            session_id: session.id.clone(),
            intent: Intent {
                name: "unknown".to_string(),
                confidence: 0.0,
                slots: vec![],
                raw_input: String::new(),
            },
            extra: HashMap::new(),
        };

        let result = registry.execute("unknown", context).await;
        drop(registry);

        let message = result.map(|r| r.message).unwrap_or_else(|e| {
            format!("抱歉，处理时出错：{}", e)
        });

        let response = DialogResponse {
            message: message.clone(),
            intent_name: None,
            confidence: None,
            state: "fallback".to_string(),
            data: None,
            follow_up: None,
        };

        self.session_manager
            .add_assistant_message(session, &message)
            .await?;

        Ok(response)
    }

    /// Describe an intent for clarification
    fn describe_intent(&self, intent: &Intent) -> String {
        let definition = self.intent_recognizer.get_intent(&intent.name);
        definition
            .map(|d| d.description.clone())
            .unwrap_or_else(|| intent.name.clone())
    }

    /// Register a custom command
    pub async fn register_command(&self, command: Arc<dyn crate::dialog::command::Command>) {
        let mut registry = self.command_registry.write().await;
        registry.register(command);
    }

    /// Get the intent recognizer (for custom intent registration)
    pub fn intent_recognizer(&self) -> &IntentRecognizer {
        &self.intent_recognizer
    }

    /// Get session manager
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// Cleanup expired sessions
    pub async fn cleanup(&self) -> Result<usize> {
        self.session_manager.cleanup_expired().await
    }

    /// Get active session count
    pub async fn active_session_count(&self) -> usize {
        self.session_manager.active_session_count().await
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_greeting() {
        let engine = DialogEngine::new(None);
        let response = engine.process_message("user1", "chat1", "test", "你好").await.unwrap();
        assert!(response.message.contains("好"));
    }

    #[tokio::test]
    async fn test_process_help() {
        let engine = DialogEngine::new(None);
        let response = engine.process_message("user1", "chat1", "test", "帮助").await.unwrap();
        assert!(response.message.contains("Desktop Agent"));
    }

    #[tokio::test]
    async fn test_process_unknown() {
        let engine = DialogEngine::new(None);
        let response = engine.process_message("user1", "chat1", "test", "xyzabc123").await.unwrap();
        assert!(!response.message.is_empty());
    }

    #[tokio::test]
    async fn test_session_tracking() {
        let engine = DialogEngine::new(None);
        let _ = engine.process_message("user1", "chat1", "test", "你好").await;
        assert_eq!(engine.active_session_count().await, 1);
    }
}
