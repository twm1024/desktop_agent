// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::error::{AppError, Result};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha1, Sha256};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

type HmacSha256 = Hmac<Sha256>;

/// Webhook signature verifier
pub struct WebhookVerifier {
    secret: String,
}

impl WebhookVerifier {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    /// Verify Feishu webhook signature
    ///
    /// Feishu uses HMAC-SHA256 with the format: `{timestamp}{nonce}{encrypt_key}{body}`
    pub fn verify_feishu(
        &self,
        timestamp: &str,
        nonce: &str,
        body: &str,
        signature: &str,
    ) -> Result<bool> {
        // Parse timestamp
        let _ts: i64 = timestamp
            .parse()
            .map_err(|_| AppError::webhook("Invalid timestamp"))?;

        // Validate timestamp (should be within 5 minutes)
        let now = chrono::Utc::now().timestamp();
        let ts = timestamp.parse::<i64>().unwrap_or(0);
        if (now - ts).abs() > 300 {
            warn!("Timestamp too old or in the future: {}", timestamp);
            return Ok(false);
        }

        // Calculate signature
        let sign_str = format!("{}{}{}{}", timestamp, nonce, self.secret, body);

        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
            .map_err(|e| AppError::security(format!("HMAC error: {}", e)))?;

        mac.update(sign_str.as_bytes());
        let expected = hex::encode(mac.finalize().into_bytes());

        // Use constant-time comparison
        Ok(hmac::compare_digest(expected.as_bytes(), signature.as_bytes()))
    }

    /// Verify WeCom (企业微信) webhook signature
    ///
    /// WeCom uses SHA1 with sorted parameters: `{token}{timestamp}{nonce}{echostr}`
    pub fn verify_wecom(
        &self,
        token: &str,
        timestamp: &str,
        nonce: &str,
        echostr: &str,
        signature: &str,
    ) -> Result<bool> {
        // Sort parameters
        let mut arr = vec![token.to_string(), timestamp.to_string(), nonce.to_string(), echostr.to_string()];
        arr.sort();

        // Calculate SHA1
        let sign_str = arr.join("");
        let mut hasher = Sha1::new();
        hasher.update(sign_str.as_bytes());
        let result = hasher.finalize();

        let expected = hex::encode(result);

        // Use constant-time comparison
        Ok(hmac::compare_digest(expected.as_bytes(), signature.as_bytes()))
    }

    /// Verify DingTalk webhook signature
    ///
    /// DingTalk uses HMAC-SHA256 with the format: `{timestamp}{secret}`
    pub fn verify_dingtalk(
        &self,
        timestamp: &str,
        secret: &str,
        signature: &str,
    ) -> Result<bool> {
        // Combine timestamp and secret
        let string_to_sign = format!("{}\n{}", timestamp, secret);

        // Calculate HMAC-SHA256
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
            .map_err(|e| AppError::security(format!("HMAC error: {}", e)))?;

        mac.update(string_to_sign.as_bytes());
        let expected = base64::encode(mac.finalize().into_bytes());

        // Use constant-time comparison
        Ok(hmac::compare_digest(expected.as_bytes(), signature.as_bytes()))
    }
}

/// Replay attack protection
pub struct ReplayProtection {
    processed_messages: Arc<RwLock<HashSet<String>>>,
    message_ttl: Duration,
    cleanup_interval: Duration,
}

impl ReplayProtection {
    pub fn new(ttl: Duration, cleanup_interval: Duration) -> Self {
        let instance = Self {
            processed_messages: Arc::new(RwLock::new(HashSet::new())),
            message_ttl: ttl,
            cleanup_interval,
        };

        // Start cleanup task
        tokio::spawn(instance.clone().cleanup_task());

        instance
    }

    /// Check if a message is a replay attack
    ///
    /// Returns Ok(true) if the message is new and valid
    /// Returns Err if the message is a duplicate or timestamp is expired
    pub async fn check(&self, message_id: &str, timestamp: i64) -> Result<bool> {
        let now = chrono::Utc::now().timestamp();

        // Check if timestamp is within valid range
        if (now - timestamp).abs() > self.message_ttl.as_secs() as i64 {
            return Err(AppError::webhook("Timestamp expired"));
        }

        // Check if message was already processed
        let mut processed = self.processed_messages.write().await;

        if processed.contains(message_id) {
            warn!("Duplicate message detected: {}", message_id);
            return Err(AppError::webhook("Duplicate message"));
        }

        // Record message
        processed.insert(message_id.to_string());

        debug!("Message accepted: {}", message_id);
        Ok(true)
    }

    async fn cleanup_task(self) {
        let mut interval = tokio::time::interval(self.cleanup_interval);

        loop {
            interval.tick().await;

            // TODO: Implement proper cleanup with timestamps
            // For now, we just clear the set periodically
            // A more sophisticated implementation would store timestamps
            // and remove messages older than TTL
            let mut processed = self.processed_messages.write().await;
            if processed.len() > 10000 {
                // Clear if too many messages
                processed.clear();
            }
        }
    }
}

/// Combined webhook security handler
pub struct WebhookSecurity {
    verifier: Arc<WebhookVerifier>,
    replay_protection: Arc<ReplayProtection>,
}

impl WebhookSecurity {
    pub fn new(secret: String) -> Self {
        Self {
            verifier: Arc::new(WebhookVerifier::new(secret)),
            replay_protection: Arc::new(ReplayProtection::new(
                Duration::from_secs(300),  // 5 minutes TTL
                Duration::from_secs(60),   // Cleanup every minute
            )),
        }
    }

    pub fn verifier(&self) -> &WebhookVerifier {
        &self.verifier
    }

    pub fn replay_protection(&self) -> &ReplayProtection {
        &self.replay_protection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feishu_verification() {
        let verifier = WebhookVerifier::new("test_secret".to_string());

        let timestamp = chrono::Utc::now().timestamp().to_string();
        let nonce = "test_nonce";
        let body = "test_body";

        // This should fail with the wrong signature
        let result = verifier.verify_feishu(&timestamp, nonce, body, "wrong_signature");
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // Calculate correct signature
        let sign_str = format!("{}{}{}{}", timestamp, nonce, "test_secret", body);
        let mut mac = HmacSha256::new_from_slice(b"test_secret").unwrap();
        mac.update(sign_str.as_bytes());
        let correct_signature = hex::encode(mac.finalize().into_bytes());

        let result = verifier.verify_feishu(&timestamp, nonce, body, &correct_signature);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_wecom_verification() {
        let verifier = WebhookVerifier::new("test_secret".to_string());

        let timestamp = "1234567890";
        let nonce = "test_nonce";
        let echostr = "test_echostr";
        let token = "test_token";

        // Calculate correct signature
        let mut arr = vec![
            token.to_string(),
            timestamp.to_string(),
            nonce.to_string(),
            echostr.to_string(),
        ];
        arr.sort();
        let sign_str = arr.join("");
        let mut hasher = Sha1::new();
        hasher.update(sign_str.as_bytes());
        let correct_signature = hex::encode(hasher.finalize());

        let result = verifier.verify_wecom(token, timestamp, nonce, echostr, &correct_signature);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_replay_protection() {
        let protection = ReplayProtection::new(Duration::from_secs(300), Duration::from_secs(60));

        let message_id = "test_message_1";
        let timestamp = chrono::Utc::now().timestamp();

        // First check should succeed
        let result = protection.check(message_id, timestamp).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Second check should fail (duplicate)
        let result = protection.check(message_id, timestamp).await;
        assert!(result.is_err());

        // Old timestamp should fail
        let old_timestamp = chrono::Utc::now().timestamp() - 400;
        let result = protection.check("test_message_2", old_timestamp).await;
        assert!(result.is_err());
    }
}
