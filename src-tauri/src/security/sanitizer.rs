// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

#![allow(dead_code)]
use once_cell::sync::Lazy;
use regex::Regex;

/// Log sanitization to remove sensitive information
pub struct LogSanitizer;

impl LogSanitizer {
    /// Sanitize log text by removing sensitive information
    pub fn sanitize(text: &str) -> String {
        let mut result = text.to_string();

        // Sanitize sensitive field patterns
        for (pattern, replacement) in SENSITIVE_FIELD_PATTERNS.iter() {
            result = pattern.replace_all(&result, *replacement).to_string();
        }

        // Sanitize sensitive info patterns
        for (pattern, replacement) in SENSITIVE_INFO_PATTERNS.iter() {
            result = pattern.replace_all(&result, *replacement).to_string();
        }

        result
    }

    /// Sanitize JSON by removing sensitive fields
    pub fn sanitize_json(json: &str) -> std::result::Result<String, serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(json)?;

        // Create a sanitized version
        let sanitized = Self::sanitize_value(&value);

        Ok(serde_json::to_string(&sanitized).unwrap_or_else(|_| sanitized.to_string()))
    }

    fn sanitize_value(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut sanitized_map = serde_json::Map::new();

                for (key, val) in map {
                    // Check if key is sensitive
                    if Self::is_sensitive_key(key) {
                        // Replace sensitive values
                        sanitized_map.insert(key.clone(), serde_json::json!("***"));
                    } else {
                        // Recursively sanitize
                        sanitized_map.insert(key.clone(), Self::sanitize_value(val));
                    }
                }

                serde_json::Value::Object(sanitized_map)
            }
            serde_json::Value::Array(arr) => {
                let sanitized_arr: Vec<serde_json::Value> = arr
                    .iter()
                    .map(|v| Self::sanitize_value(v))
                    .collect();
                serde_json::Value::Array(sanitized_arr)
            }
            _ => value.clone(),
        }
    }

    /// Check if a key name indicates sensitive data
    fn is_sensitive_key(key: &str) -> bool {
        let key_lower = key.to_lowercase();

        SENSITIVE_KEYS.contains(&key_lower.as_str())
    }
}

// Sensitive field name patterns
static SENSITIVE_FIELD_PATTERNS: Lazy<Vec<(Regex, &str)>> = Lazy::new(|| {
    vec![
        (Regex::new(r###"password['"]?\s*[:=]\s*['"]?[^'"]+"###).unwrap(), "password=***"),
        (Regex::new(r###"passwd['"]?\s*[:=]\s*['"]?[^'"]+"###).unwrap(), "passwd=***"),
        (Regex::new(r###"pwd['"]?\s*[:=]\s*['"]?[^'"]+"###).unwrap(), "pwd=***"),
        (Regex::new(r###"token['"]?\s*[:=]\s*['"]?[^'"]{10,}"###).unwrap(), "token=***"),
        (Regex::new(r###"access_token['"]?\s*[:=]\s*['"]?[^'"]{10,}"###).unwrap(), "access_token=***"),
        (Regex::new(r###"refresh_token['"]?\s*[:=]\s*['"]?[^'"]{10,}"###).unwrap(), "refresh_token=***"),
        (Regex::new(r###"secret['"]?\s*[:=]\s*['"]?[^'"]{10,}"###).unwrap(), "secret=***"),
        (Regex::new(r###"api_key['"]?\s*[:=]\s*['"]?[^'"]{10,}"###).unwrap(), "api_key=***"),
        (Regex::new(r###"apikey['"]?\s*[:=]\s*['"]?[^'"]{10,}"###).unwrap(), "apikey=***"),
        (Regex::new(r###"app_secret['"]?\s*[:=]\s*['"]?[^'"]{10,}"###).unwrap(), "app_secret=***"),
        (Regex::new(r###"appkey['"]?\s*[:=]\s*['"]?[^'"]{10,}"###).unwrap(), "appkey=***"),
        (Regex::new(r###"private_key['"]?\s*[:=]\s*['"]?[^'"]{20,}"###).unwrap(), "private_key=***"),
        (Regex::new(r###"privatekey['"]?\s*[:=]\s*['"]?[^'"]{20,}"###).unwrap(), "privatekey=***"),
        (Regex::new(r###"authorization['"]?\s*[:=]\s*['"]?[^'"]{10,}"###).unwrap(), "authorization=***"),
        (Regex::new(r###"auth['"]?\s*[:=]\s*['"]?[^'"]{10,}"###).unwrap(), "auth=***"),
        (Regex::new(r###"bearer['"]?\s*[:=]\s*['"]?[^'"]{10,}"###).unwrap(), "bearer=***"),
    ]
});

// Sensitive information patterns
static SENSITIVE_INFO_PATTERNS: Lazy<Vec<(Regex, &str)>> = Lazy::new(|| {
    vec![
        (Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap(), "[email]"),
        (Regex::new(r"\b1[3-9]\d{9}\b").unwrap(), "[phone]"),
        (Regex::new(r"\b[1-9]\d{5}(18|19|20)\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])\d{3}[\dXx]\b").unwrap(), "[id_card]"),
        (Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap(), "[ip]"),
        (Regex::new(r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b").unwrap(), "[uuid]"),
    ]
});

// Sensitive key names
static SENSITIVE_KEYS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "token",
    "access_token",
    "refresh_token",
    "api_key",
    "apikey",
    "app_secret",
    "appkey",
    "private_key",
    "privatekey",
    "authorization",
    "auth",
    "bearer",
    "credential",
    "credit_card",
    "creditcard",
    "ssn",
    "social_security",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_password() {
        let input = "User logged in with password='MySecretPass123'";
        let sanitized = LogSanitizer::sanitize(input);

        assert!(!sanitized.contains("MySecretPass123"));
        assert!(sanitized.contains("password=***"));
    }

    #[test]
    fn test_sanitize_token() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let sanitized = LogSanitizer::sanitize(input);

        assert!(!sanitized.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
        assert!(sanitized.contains("***"));
    }

    #[test]
    fn test_sanitize_email() {
        let input = "User email: user@example.com";
        let sanitized = LogSanitizer::sanitize(input);

        assert!(!sanitized.contains("user@example.com"));
        assert!(sanitized.contains("[email]"));
    }

    #[test]
    fn test_sanitize_phone() {
        let input = "Contact number: 13812345678";
        let sanitized = LogSanitizer::sanitize(input);

        assert!(!sanitized.contains("13812345678"));
        assert!(sanitized.contains("[phone]"));
    }

    #[test]
    fn test_sanitize_ip() {
        let input = "Request from 192.168.1.1";
        let sanitized = LogSanitizer::sanitize(input);

        assert!(!sanitized.contains("192.168.1.1"));
        assert!(sanitized.contains("[ip]"));
    }

    #[test]
    fn test_sanitize_json() {
        let input = r#"{"username": "user1", "password": "secret123", "email": "user@example.com"}"#;
        let sanitized = LogSanitizer::sanitize_json(input).unwrap();

        let value: serde_json::Value = serde_json::from_str(&sanitized).unwrap();
        assert_eq!(value["username"], "user1");
        assert_eq!(value["password"], "***");
        assert_eq!(value["email"], "[email]");
    }

    #[test]
    fn test_sanitize_multiple_secrets() {
        let input = "Config: api_key=abc123def456, secret=xyz789, token=token123";
        let sanitized = LogSanitizer::sanitize(input);

        assert!(!sanitized.contains("abc123def456"));
        assert!(!sanitized.contains("xyz789"));
        assert!(!sanitized.contains("token123"));
    }

    #[test]
    fn test_sanitize_preserves_safe_data() {
        let input = "User: john_doe, Action: read_file, Path: /home/user/docs";
        let sanitized = LogSanitizer::sanitize(input);

        assert!(sanitized.contains("john_doe"));
        assert!(sanitized.contains("read_file"));
        assert!(sanitized.contains("/home/user/docs"));
    }

    #[test]
    fn test_sanitize_id_card() {
        let input = "ID Card: 110101199001011234";
        let sanitized = LogSanitizer::sanitize(input);

        assert!(!sanitized.contains("110101199001011234"));
        assert!(sanitized.contains("[id_card]"));
    }

    #[test]
    fn test_sanitize_uuid() {
        let input = "Session ID: 550e8400-e29b-41d4-a716-446655440000";
        let sanitized = LogSanitizer::sanitize(input);

        assert!(!sanitized.contains("550e8400-e29b-41d4-a716-446655440000"));
        assert!(sanitized.contains("[uuid]"));
    }
}
