// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Network service for making HTTP requests
//!
//! Provides a secure HTTP client with timeout, retry, and rate limiting support

use crate::error::{AppError, Result};
use crate::security::rate_limiter::RateLimiter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// HTTP request method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "uppercase")]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::OPTIONS => "OPTIONS",
        }
    }
}

/// HTTP request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub url: String,
    pub method: HttpMethod,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<serde_json::Value>,
    pub query: Option<HashMap<String, String>>,
    pub timeout: Option<u64>, // milliseconds
    pub max_redirects: Option<usize>,
}

/// HTTP response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub duration_ms: u64,
    pub success: bool,
}

impl HttpResponse {
    /// Parse response body as JSON
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        serde_json::from_str(&self.body)
            .map_err(|e| AppError::Serialization(format!("Failed to parse JSON: {}", e)))
    }
}

/// Network service configuration
#[derive(Debug, Clone)]
pub struct NetworkServiceConfig {
    pub timeout: Duration,
    pub max_redirects: usize,
    pub user_agent: String,
    pub max_concurrent_requests: usize,
    pub enable_rate_limit: bool,
    pub rate_limit_per_second: usize,
}

impl Default for NetworkServiceConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_redirects: 10,
            user_agent: "Desktop-Agent/0.1.0".to_string(),
            max_concurrent_requests: 50,
            enable_rate_limit: true,
            rate_limit_per_second: 100,
        }
    }
}

/// Network service
pub struct NetworkService {
    config: NetworkServiceConfig,
    client: reqwest::Client,
    rate_limiter: Option<Arc<RateLimiter>>,
    semaphore: Arc<Semaphore>,
}

impl NetworkService {
    pub fn new(config: NetworkServiceConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::limited(config.max_redirects))
            .user_agent(&config.user_agent)
            .build()
            .map_err(|e| AppError::Network(format!("Failed to create HTTP client: {}", e)))?;

        let rate_limiter = if config.enable_rate_limit {
            Some(Arc::new(RateLimiter::new(
                config.rate_limit_per_second,
                config.rate_limit_per_second * 2,
                Duration::from_secs(1),
            )))
        } else {
            None
        };

        Ok(Self {
            config,
            client,
            rate_limiter,
            semaphore: Arc::new(Semaphore::new(config.max_concurrent_requests)),
        })
    }

    /// Execute an HTTP request
    pub async fn request(&self, request: HttpRequest) -> Result<HttpResponse> {
        // Acquire semaphore slot
        let _permit = self.semaphore.acquire().await
            .map_err(|e| AppError::Network(format!("Semaphore error: {}", e)))?;

        // Check rate limit
        if let Some(rate_limiter) = &self.rate_limiter {
            let key = "network_global";
            if !rate_limiter.check(key, 1).await {
                return Err(AppError::RateLimit("Rate limit exceeded".to_string()));
            }
        }

        let start = std::time::Instant::now();

        // Build request
        let mut req_builder = match request.method {
            HttpMethod::GET => self.client.get(&request.url),
            HttpMethod::POST => self.client.post(&request.url),
            HttpMethod::PUT => self.client.put(&request.url),
            HttpMethod::DELETE => self.client.delete(&request.url),
            HttpMethod::PATCH => self.client.patch(&request.url),
            HttpMethod::HEAD => self.client.head(&request.url),
            HttpMethod::OPTIONS => self.client.request(reqwest::Method::OPTIONS, &request.url),
        };

        // Add query parameters
        if let Some(query) = request.query {
            req_builder = req_builder.query(&query);
        }

        // Add headers
        if let Some(headers) = request.headers {
            for (key, value) in headers {
                req_builder = req_builder.header(key, value);
            }
        }

        // Add body
        if let Some(body) = request.body {
            req_builder = req_builder.json(&body);
        }

        // Apply timeout if specified
        let req_builder = if let Some(timeout_ms) = request.timeout {
            req_builder.timeout(Duration::from_millis(timeout_ms))
        } else {
            req_builder
        };

        // Execute request
        let response = req_builder.send().await
            .map_err(|e| AppError::Network(format!("Request failed: {}", e)))?;

        let status = response.status();
        let status_text = status.canonical_reason().unwrap_or("Unknown").to_string();

        // Get response headers
        let mut headers = HashMap::new();
        for (name, value) in response.headers().iter() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.as_str().to_string(), value_str.to_string());
            }
        }

        // Get response body
        let body = response.text().await
            .map_err(|e| AppError::Network(format!("Failed to read response body: {}", e)))?;

        let duration = start.elapsed();

        Ok(HttpResponse {
            status: status.as_u16(),
            status_text,
            headers,
            body,
            duration_ms: duration.as_millis() as u64,
            success: status.is_success(),
        })
    }

    /// Execute a GET request
    pub async fn get(&self, url: String) -> Result<HttpResponse> {
        self.request(HttpRequest {
            url,
            method: HttpMethod::GET,
            headers: None,
            body: None,
            query: None,
            timeout: None,
            max_redirects: None,
        }).await
    }

    /// Execute a POST request
    pub async fn post(&self, url: String, body: serde_json::Value) -> Result<HttpResponse> {
        self.request(HttpRequest {
            url,
            method: HttpMethod::POST,
            headers: None,
            body: Some(body),
            query: None,
            timeout: None,
            max_redirects: None,
        }).await
    }

    /// Download a file
    pub async fn download(&self, url: String) -> Result<Vec<u8>> {
        let _permit = self.semaphore.acquire().await
            .map_err(|e| AppError::Network(format!("Semaphore error: {}", e)))?;

        let response = self.client.get(&url)
            .send().await
            .map_err(|e| AppError::Network(format!("Download failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::Network(format!("Download failed with status: {}", response.status())));
        }

        let bytes = response.bytes().await
            .map_err(|e| AppError::Network(format!("Failed to download bytes: {}", e)))?;

        Ok(bytes.to_vec())
    }

    /// Upload a file
    pub async fn upload(&self, url: String, data: Vec<u8>, mime_type: Option<String>) -> Result<HttpResponse> {
        let _permit = self.semaphore.acquire().await
            .map_err(|e| AppError::Network(format!("Semaphore error: {}", e)))?;

        let part = reqwest::multipart::Part::bytes(data)
            .file_name("file");

        let part = if let Some(mime) = mime_type {
            part.mime_str(&mime)
                .map_err(|e| AppError::Network(format!("Invalid MIME type: {}", e)))?
        } else {
            part
        };

        let form = reqwest::multipart::Form::new().part("file", part);

        let response = self.client.post(&url)
            .multipart(form)
            .send().await
            .map_err(|e| AppError::Network(format!("Upload failed: {}", e)))?;

        let status = response.status();
        let body = response.text().await
            .map_err(|e| AppError::Network(format!("Failed to read response: {}", e)))?;

        Ok(HttpResponse {
            status: status.as_u16(),
            status_text: status.canonical_reason().unwrap_or("Unknown").to_string(),
            headers: HashMap::new(),
            body,
            duration_ms: 0,
            success: status.is_success(),
        })
    }
}

impl Default for NetworkService {
    fn default() -> Self {
        Self::new(NetworkServiceConfig::default()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_method_as_str() {
        assert_eq!(HttpMethod::GET.as_str(), "GET");
        assert_eq!(HttpMethod::POST.as_str(), "POST");
    }

    #[tokio::test]
    async fn test_network_service_config() {
        let config = NetworkServiceConfig::default();
        assert_eq!(config.timeout.as_secs(), 30);
        assert_eq!(config.max_redirects, 10);
    }
}
