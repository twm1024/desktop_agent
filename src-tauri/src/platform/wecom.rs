// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! WeCom (企业微信/WeCom) platform integration
//!
//! Provides webhook handling and API integration for WeCom platform

use async_trait::async_trait;
use crate::platform::adapter::{PlatformAdapter, PlatformEvent, PlatformResponse, PlatformUser, PlatformMessage, ResponseMessage, ChatInfo, ChatType, MediaType, PlatformError};
use crate::platform::PlatformType;
use crate::security::webhook::WebhookVerifier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// WeCom platform adapter
pub struct WeComAdapter {
    corp_id: String,
    token: String,
    aes_key: String,
    webhook_verifier: WebhookVerifier,
    http_client: reqwest::Client,
}

impl WeComAdapter {
    /// Create a new WeCom adapter
    pub fn new(
        corp_id: String,
        token: String,
        aes_key: String,
    ) -> Self {
        let webhook_verifier = WebhookVerifier::new_with_token(token.clone());

        Self {
            corp_id,
            token,
            aes_key,
            webhook_verifier,
            http_client: reqwest::Client::new(),
        }
    }

    /// Get WeCom API access token
    async fn get_access_token(&self) -> Result<String, PlatformError> {
        #[derive(Deserialize)]
        struct AccessTokenResponse {
            errcode: i32,
            errmsg: String,
            access_token: String,
        }

        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/gettoken?corpid={}&secret={}",
            self.corp_id, self.token
        );

        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("Failed to get access token: {}", e)))?;

        let data: AccessTokenResponse = response
            .json()
            .await
            .map_err(|e| PlatformError::ParseError(format!("Failed to parse response: {}", e)))?;

        if data.errcode != 0 {
            return Err(PlatformError::ApiError(format!(
                "Failed to get access token: {} (code: {})",
                data.errmsg, data.errcode
            )));
        }

        Ok(data.access_token)
    }

    /// Send message to WeCom
    async fn send_message_wecom(
        &self,
        chat_id: &str,
        message: &ResponseMessage,
    ) -> Result<(), PlatformError> {
        let token = self.get_access_token().await?;

        #[derive(Serialize)]
        struct SendMessageRequest {
            touser: Option<String>,
            chatid: Option<String>,
            msgtype: String,
            text: Option<TextContent>,
            markdown: Option<MarkdownContent>,
            textcard: Option<TextCardContent>,
        }

        #[derive(Serialize)]
        struct TextContent {
            content: String,
        }

        #[derive(Serialize)]
        struct MarkdownContent {
            content: String,
        }

        #[derive(Serialize)]
        struct TextCardContent {
            title: String,
            description: String,
            url: String,
            btntxt: Option<String>,
        }

        let (msg_type, text, markdown, textcard) = match message {
            ResponseMessage::Text { content } => {
                ("text".to_string(),
                 Some(TextContent { content: content.clone() }),
                 None,
                 None)
            }
            ResponseMessage::Markdown { content } => {
                ("markdown".to_string(),
                 None,
                 Some(MarkdownContent { content: content.clone() }),
                 None)
            }
            ResponseMessage::Card { elements } => {
                // Convert card to text card format
                let title = elements.first()
                    .and_then(|e| e.content.get("title"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("消息")
                    .to_string();

                let description = elements.iter()
                    .filter_map(|e| e.content.get("content"))
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");

                ("textcard".to_string(),
                 None,
                 None,
                 Some(TextCardContent {
                     title,
                     description,
                     url: "".to_string(),
                     btntxt: Some("详情".to_string()),
                 }))
            }
            _ => {
                return Err(PlatformError::UnsupportedOperation(
                    "Message type not supported".to_string(),
                ))
            }
        };

        let request = SendMessageRequest {
            touser: None,
            chatid: Some(chat_id.to_string()),
            msgtype,
            text,
            markdown,
            textcard,
        };

        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token={}",
            token
        );

        let response = self.http_client
            .post(&url)
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
impl PlatformAdapter for WeComAdapter {
    fn platform_type(&self) -> PlatformType {
        PlatformType::WeCom
    }

    async fn parse_event(
        &self,
        _headers: &HashMap<String, String>,
        body: &str,
    ) -> Result<PlatformEvent, PlatformError> {
        // Parse WeCom event JSON
        let event_data: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| PlatformError::ParseError(format!("Failed to parse JSON: {}", e)))?;

        // Extract event data
        let event_type = event_data
            .get("eventtype")
            .or_else(|| event_data.get("event"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let timestamp = event_data
            .get("createtime")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| chrono::Utc::now().timestamp());

        // Extract user information
        let user_id = event_data
            .get("fromusername")
            .or_else(|| event_data.get("userid"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| PlatformError::ParseError("Missing user_id".to_string()))?
            .to_string();

        let user_name = event_data
            .get("fromusername")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let chat_id = event_data
            .get("to_username")
            .or_else(|| event_data.get("chatid"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| PlatformError::ParseError("Missing chat_id".to_string()))?
            .to_string();

        // Extract message content
        let message = event_data.get("content")
            .and_then(|c| c.as_str())
            .map(|content| {
                PlatformMessage::Text {
                    content: content.to_string()
                }
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
            platform: PlatformType::WeCom,
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
        self.send_message_wecom(&response.chat_id, &response.message)
            .await?;

        Ok(())
    }

    async fn verify_webhook(
        &self,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> Result<bool, PlatformError> {
        // WeCom uses different signature verification
        let msg_signature = headers
            .get("msg_signature")
            .or_else(|| headers.get("signature"))
            .map(|s| s.as_str())
            .ok_or_else(|| PlatformError::VerificationFailed("Missing signature".to_string()))?;

        // Extract timestamp and nonce from URL parameters or headers
        let timestamp = headers
            .get("timestamp")
            .map(|s| s.as_str())
            .unwrap_or("0");

        let nonce = headers
            .get("nonce")
            .map(|s| s.as_str())
            .unwrap_or("");

        // Verify signature
        self.webhook_verifier
            .verify_wecom(timestamp, nonce, body, msg_signature, &self.aes_key)
            .map_err(|e| PlatformError::VerificationFailed(e.to_string()))
    }

    async fn get_user(&self, user_id: &str) -> Result<PlatformUser, PlatformError> {
        let token = self.get_access_token().await?;

        #[derive(Deserialize)]
        struct UserInfoResponse {
            errcode: i32,
            errmsg: String,
            userid: Option<String>,
            name: Option<String>,
            avatar: Option<String>,
            email: Option<String>,
            mobile: Option<String>,
            department: Option<Vec<Department>>,
        }

        #[derive(Deserialize)]
        struct Department {
            name: Option<String>,
        }

        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/user/get?access_token={}&userid={}",
            token, user_id
        );

        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("Failed to get user: {}", e)))?;

        let data: UserInfoResponse = response
            .json()
            .await
            .map_err(|e| PlatformError::ParseError(format!("Failed to parse response: {}", e)))?;

        if data.errcode != 0 {
            return Err(PlatformError::ApiError(format!(
                "Failed to get user: {} (code: {})",
                data.errmsg, data.errcode
            )));
        }

        Ok(PlatformUser {
            user_id: data.userid.unwrap_or_else(|| user_id.to_string()),
            name: data.name,
            avatar: data.avatar,
            email: data.email,
            mobile: data.mobile,
            department: data.department.and_then(|depts| depts.first().and_then(|d| d.name.clone())),
            extra: HashMap::new(),
        })
    }

    async fn get_chat(&self, chat_id: &str) -> Result<ChatInfo, PlatformError> {
        let token = self.get_access_token().await?;

        #[derive(Deserialize)]
        struct ChatInfoResponse {
            errcode: i32,
            errmsg: String,
            chatid: Option<String>,
            name: Option<String>,
            owner: Option<String>,
            member_list: Option<Vec<Member>>,
        }

        #[derive(Deserialize)]
        struct Member {
            userid: Option<String>,
        }

        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/externalcontact/group_chat/get?access_token={}&chat_id={}",
            token, chat_id
        );

        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| PlatformError::NetworkError(format!("Failed to get chat: {}", e)))?;

        let data: ChatInfoResponse = response
            .json()
            .await
            .map_err(|e| PlatformError::ParseError(format!("Failed to parse response: {}", e)))?;

        if data.errcode != 0 {
            return Err(PlatformError::ApiError(format!(
                "Failed to get chat: {} (code: {})",
                data.errmsg, data.errcode
            )));
        }

        Ok(ChatInfo {
            chat_id: chat_id.to_string(),
            name: data.name,
            chat_type: ChatType::Group,
            owner_id: data.owner,
            member_count: data.member_list.map(|m| m.len()),
        })
    }

    async fn upload_media(
        &self,
        _file_path: &str,
        _file_type: MediaType,
    ) -> Result<String, PlatformError> {
        Err(PlatformError::UnsupportedOperation(
            "Media upload not implemented for WeCom".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wecom_adapter_creation() {
        let adapter = WeComAdapter::new(
            "corp_id".to_string(),
            "token".to_string(),
            "aes_key".to_string(),
        );
        assert_eq!(adapter.platform_type(), PlatformType::WeCom);
    }
}
