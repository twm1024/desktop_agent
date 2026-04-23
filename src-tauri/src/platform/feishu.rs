// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Feishu (Lark) platform integration
//!
//! Provides webhook handling and API integration for Feishu/Lark platform

#![allow(dead_code)]
use async_trait::async_trait;
use crate::platform::adapter::{PlatformAdapter, PlatformEvent, PlatformResponse, PlatformUser, PlatformMessage, ResponseMessage, ChatInfo, ChatType, MediaType, PlatformError};
use crate::platform::PlatformType;
use crate::security::webhook::WebhookVerifier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Feishu platform adapter
pub struct FeishuAdapter {
    app_id: String,
    app_secret: String,
    encrypt_key: String,
    verification_token: String,
    webhook_verifier: WebhookVerifier,
    http_client: reqwest::Client,
}

impl FeishuAdapter {
    /// Create a new Feishu adapter
    pub fn new(
        app_id: String,
        app_secret: String,
        encrypt_key: String,
        verification_token: String,
    ) -> Self {
        let webhook_verifier = WebhookVerifier::new(encrypt_key.clone());

        Self {
            app_id,
            app_secret,
            encrypt_key,
            verification_token,
            webhook_verifier,
            http_client: reqwest::Client::new(),
        }
    }

    /// Get Feishu API access token
    async fn get_access_token(&self) -> Result<String, PlatformError> {
        #[derive(Serialize)]
        struct TenantAccessTokenRequest {
            app_id: String,
            app_secret: String,
        }

        #[derive(Deserialize)]
        struct TenantAccessTokenResponse {
            code: i32,
            tenant_access_token: String,
        }

        let request = TenantAccessTokenRequest {
            app_id: self.app_id.clone(),
            app_secret: self.app_secret.clone(),
        };

        let response = self.http_client
            .post("https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal")
            .json(&request)
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("Failed to get access token: {}", e)))?;

        let data: TenantAccessTokenResponse = response
            .json()
            .await
            .map_err(|e| PlatformError::ParseError(format!("Failed to parse response: {}", e)))?;

        if data.code != 0 {
            return Err(PlatformError::ApiError(format!("Failed to get access token: code {}", data.code)));
        }

        Ok(data.tenant_access_token)
    }

    /// Send message to Feishu
    async fn send_message_feishu(
        &self,
        chat_id: &str,
        message: &ResponseMessage,
        msg_type: &str,
    ) -> Result<(), PlatformError> {
        let token = self.get_access_token().await?;

        #[derive(Serialize)]
        struct SendMessageRequest {
            msg_type: String,
            receive_id_type: String,
            receive_id: String,
            content: String,
        }

        let content = match message {
            ResponseMessage::Text { content } => {
                serde_json::json!({ "text": content })
            }
            ResponseMessage::Markdown { content } => {
                serde_json::json!({ "text": content })
            }
            ResponseMessage::Card { elements } => {
                serde_json::json!({ "elements": elements })
            }
            _ => {
                return Err(PlatformError::UnsupportedOperation(
                    "Message type not supported".to_string(),
                ))
            }
        };

        let request = SendMessageRequest {
            msg_type: msg_type.to_string(),
            receive_id_type: "chat_id".to_string(),
            receive_id: chat_id.to_string(),
            content: serde_json::to_string(&content)
                .map_err(|e| PlatformError::ParseError(format!("Failed to serialize content: {}", e)))?,
        };

        let response = self
            .http_client
            .post(format!(
                "https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=chat_id"
            ))
            .header("Authorization", format!("Bearer {}", token))
            .json(&request)
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("Failed to send message: {}", e)))?;

        if !response.status().is_success() {
            return Err(PlatformError::ApiError(format!(
                "Failed to send message: {}",
                response.status()
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl PlatformAdapter for FeishuAdapter {
    fn platform_type(&self) -> PlatformType {
        PlatformType::Feishu
    }

    async fn parse_event(
        &self,
        _headers: &HashMap<String, String>,
        body: &str,
    ) -> Result<PlatformEvent, PlatformError> {
        // Parse Feishu event JSON
        let event_data: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| PlatformError::ParseError(format!("Failed to parse JSON: {}", e)))?;

        // Extract event data
        let event_type = event_data
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let timestamp = event_data
            .get("timestamp")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| chrono::Utc::now().timestamp());

        // Extract user information
        let user_data = event_data.get("event")
            .and_then(|e| e.get("sender"))
            .and_then(|s| s.get("sender_id"));

        let user_id = user_data
            .and_then(|u| u.get("open_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| PlatformError::ParseError("Missing user_id".to_string()))?
            .to_string();

        let user_name = user_data
            .and_then(|u| u.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let chat_id = event_data
            .get("event")
            .and_then(|e| e.get("chat"))
            .and_then(|c| {
                c.get("chat_id")
                    .or_else(|| c.get("open_chat_id"))
            })
            .and_then(|v| v.as_str())
            .ok_or_else(|| PlatformError::ParseError("Missing chat_id".to_string()))?
            .to_string();

        // Extract message content
        let message = event_data.get("event")
            .and_then(|e| e.get("message"))
            .and_then(|m| {
                let content = m.get("content")?.as_str()?;
                let msg_type = m.get("message_type")?.as_str()?;

                let message_data = match msg_type {
                    "text" => {
                        serde_json::from_str::<FeishuTextContent>(content).ok()
                            .map(|data| PlatformMessage::Text { content: data.text })
                    }
                    "image" => {
                        serde_json::from_str::<FeishuImageContent>(content).ok()
                            .map(|data| PlatformMessage::Image {
                                url: data.image_key,
                                alt: None,
                            })
                    }
                    "file" => {
                        serde_json::from_str::<FeishuFileContent>(content).ok()
                            .map(|data| PlatformMessage::File {
                                url: data.file_key,
                                name: "file".to_string(),
                                size: None,
                            })
                    }
                    _ => None,
                };

                message_data
            });

        let user = PlatformUser {
            user_id,
            name: user_name,
            avatar: None,
            email: None,
            mobile: None,
            department: None,
            extra: HashMap::new(),
        };

        Ok(PlatformEvent {
            platform: PlatformType::Feishu,
            event_type,
            user,
            chat_id,
            message,
            timestamp,
            extra: HashMap::new(),
        })
    }

    async fn send_response(
        &self,
        response: &PlatformResponse,
    ) -> Result<(), PlatformError> {
        let msg_type = match &response.message {
            ResponseMessage::Text { .. } => "text",
            ResponseMessage::Markdown { .. } => "interactive",
            ResponseMessage::Card { .. } => "interactive",
            ResponseMessage::Image { .. } => "image",
            ResponseMessage::File { .. } => "file",
        };

        self.send_message_feishu(&response.chat_id, &response.message, msg_type)
            .await?;

        Ok(())
    }

    async fn verify_webhook(
        &self,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> Result<bool, PlatformError> {
        // Extract verification headers
        let timestamp = headers
            .get("X-Lark-Request-Timestamp")
            .or_else(|| headers.get("x-lark-request-timestamp"))
            .map(|s| s.as_str())
            .ok_or_else(|| PlatformError::VerificationFailed("Missing timestamp".to_string()))?;

        let nonce = headers
            .get("X-Lark-Request-Nonce")
            .or_else(|| headers.get("x-lark-request-nonce"))
            .map(|s| s.as_str())
            .ok_or_else(|| PlatformError::VerificationFailed("Missing nonce".to_string()))?;

        let signature = headers
            .get("X-Lark-Signature")
            .or_else(|| headers.get("x-lark-signature"))
            .map(|s| s.as_str())
            .ok_or_else(|| PlatformError::VerificationFailed("Missing signature".to_string()))?;

        // Verify signature
        self.webhook_verifier
            .verify_feishu(timestamp, nonce, body, signature)
            .map_err(|e| PlatformError::VerificationFailed(e.to_string()))
    }

    async fn get_user(&self, user_id: &str) -> Result<PlatformUser, PlatformError> {
        let token = self.get_access_token().await?;

        #[derive(Deserialize)]
        struct UserInfoResponse {
            code: i32,
            data: Option<UserInfoData>,
        }

        #[derive(Deserialize)]
        struct UserInfoData {
            user: Option<FeishuUser>,
        }

        #[derive(Deserialize)]
        struct FeishuUser {
            name: Option<String>,
            avatar: Option<String>,
            email: Option<String>,
            mobile: Option<String>,
            department_ids: Option<Vec<String>>,
        }

        let response = self
            .http_client
            .get(format!(
                "https://open.feishu.cn/open-apis/contact/v3/users/{}",
                user_id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .query(&[("user_id_type", "open_id")])
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("Failed to get user: {}", e)))?;

        let data: UserInfoResponse = response
            .json()
            .await
            .map_err(|e| PlatformError::ParseError(format!("Failed to parse response: {}", e)))?;

        if data.code != 0 {
            return Err(PlatformError::ApiError(format!("Failed to get user: code {}", data.code)));
        }

        let user_info = data
            .data
            .and_then(|d| d.user)
            .ok_or_else(|| PlatformError::ApiError("User not found".to_string()))?;

        Ok(PlatformUser {
            user_id: user_id.to_string(),
            name: user_info.name,
            avatar: user_info.avatar,
            email: user_info.email,
            mobile: user_info.mobile,
            department: user_info.department_ids.and_then(|ids| ids.first().cloned()),
            extra: HashMap::new(),
        })
    }

    async fn get_chat(&self, chat_id: &str) -> Result<ChatInfo, PlatformError> {
        let token = self.get_access_token().await?;

        #[derive(Deserialize)]
        struct ChatInfoResponse {
            code: i32,
            data: Option<ChatInfoData>,
        }

        #[derive(Deserialize)]
        struct ChatInfoData {
            name: Option<String>,
            chat_type: Option<String>,
            owner_id: Option<String>,
        }

        let response = self
            .http_client
            .get(format!(
                "https://open.feishu.cn/open-apis/im/v1/chats/{}",
                chat_id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .query(&[("chat_id_type", "chat_id")])
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("Failed to get chat: {}", e)))?;

        let data: ChatInfoResponse = response
            .json()
            .await
            .map_err(|e| PlatformError::ParseError(format!("Failed to parse response: {}", e)))?;

        if data.code != 0 {
            return Err(PlatformError::ApiError(format!("Failed to get chat: code {}", data.code)));
        }

        let chat_info = data
            .data
            .ok_or_else(|| PlatformError::ApiError("Chat not found".to_string()))?;

        let chat_type = match chat_info.chat_type.as_deref() {
            Some("p2p") => ChatType::Private,
            Some("group") => ChatType::Group,
            Some("bot") => ChatType::Bot,
            _ => ChatType::Private,
        };

        Ok(ChatInfo {
            chat_id: chat_id.to_string(),
            name: chat_info.name,
            chat_type,
            owner_id: chat_info.owner_id,
            member_count: None,
        })
    }

    async fn upload_media(
        &self,
        _file_path: &str,
        _file_type: MediaType,
    ) -> Result<String, PlatformError> {
        Err(PlatformError::UnsupportedOperation(
            "Media upload not implemented for Feishu".to_string(),
        ))
    }

    async fn download_media(&self, url: &str) -> Result<Vec<u8>, PlatformError> {
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("Failed to download: {}", e)))?;

        if !response.status().is_success() {
            return Err(PlatformError::ApiError(format!(
                "Failed to download: {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("Failed to read bytes: {}", e)))?;

        Ok(bytes.to_vec())
    }

    async fn health_check(&self) -> Result<(), PlatformError> {
        self.get_access_token().await?;
        Ok(())
    }
}

/// Feishu text message content
#[derive(Debug, Deserialize)]
struct FeishuTextContent {
    text: String,
}

/// Feishu image message content
#[derive(Debug, Deserialize)]
struct FeishuImageContent {
    image_key: String,
}

/// Feishu file message content
#[derive(Debug, Deserialize)]
struct FeishuFileContent {
    file_key: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feishu_adapter_creation() {
        let adapter = FeishuAdapter::new(
            "app_id".to_string(),
            "app_secret".to_string(),
            "encrypt_key".to_string(),
            "verification_token".to_string(),
        );
        assert_eq!(adapter.platform_type(), PlatformType::Feishu);
    }
}
