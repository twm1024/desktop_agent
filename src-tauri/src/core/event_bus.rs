// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    // Skill events
    SkillLoaded { skill_id: String },
    SkillUnloaded { skill_id: String },
    SkillExecuted { skill_id: String, result: String },
    SkillFailed { skill_id: String, error: String },

    // Dialog events
    MessageReceived { user_id: String, content: String },
    IntentRecognized { intent: String, confidence: f32 },
    DialogStarted { session_id: String },
    DialogEnded { session_id: String },

    // System events
    ConfigChanged,
    ServiceStarted { service_name: String },
    ServiceStopped { service_name: String },
    ErrorOccurred { source: String, error: String },
}

pub struct EventBus {
    sender: broadcast::Sender<AppEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { sender }
    }

    pub fn publish(&self, event: AppEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        bus.publish(AppEvent::ConfigChanged);

        let event = rx.blocking_recv().unwrap();
        matches!(event, AppEvent::ConfigChanged);
    }
}
