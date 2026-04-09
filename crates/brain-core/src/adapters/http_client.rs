//! Shared HTTP client for API calls

use crate::error::{Error, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// HTTP client configuration
#[derive(Clone)]
pub struct HttpClientConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
}

impl HttpClientConfig {
    /// Create a new config with optional Bearer auth
    pub fn new(endpoint: &str, api_key: Option<&str>) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            api_key: api_key.map(|k| k.to_string()),
        }
    }
}

/// Shared HTTP client implementing common POST logic
#[derive(Clone)]
pub struct HttpClient {
    config: HttpClientConfig,
}

impl HttpClient {
    /// Create a new HTTP client with the given config
    pub fn new(config: HttpClientConfig) -> Self {
        Self { config }
    }

    /// POST request with JSON body, returns parsed JSON response
    pub fn post<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        let url = format!("{}/{}", self.config.endpoint, path);
        let body_str =
            serde_json::to_string(body).map_err(|e| Error::Config(format!("JSON error: {}", e)))?;

        let mut request = ureq::post(&url).set("Content-Type", "application/json");

        if let Some(ref key) = self.config.api_key {
            request = request.set("Authorization", &format!("Bearer {}", key));
        }

        let response = request
            .send_string(&body_str)
            .map_err(|e| Error::Http(format!("Request failed: {}", e)))?;

        if response.status() >= 400 {
            return Err(Error::Http(format!("HTTP error: {}", response.status())));
        }

        let text = response.into_string().map_err(Error::Io)?;
        serde_json::from_str(&text).map_err(|e| Error::Config(format!("Parse error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_config_new() {
        let config = HttpClientConfig::new("https://api.example.com", Some("key123"));
        assert_eq!(config.endpoint, "https://api.example.com");
        assert_eq!(config.api_key, Some("key123".to_string()));
    }

    #[test]
    fn test_http_client_config_trims_trailing_slash() {
        let config = HttpClientConfig::new("https://api.example.com/", None);
        assert_eq!(config.endpoint, "https://api.example.com");
    }

    #[test]
    fn test_http_client_config_without_api_key() {
        let config = HttpClientConfig::new("http://localhost:8080", None);
        assert_eq!(config.endpoint, "http://localhost:8080");
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_http_client_config_with_empty_api_key() {
        let config = HttpClientConfig::new("http://localhost:8080", Some(""));
        assert!(config.api_key.is_some());
        assert_eq!(config.api_key, Some("".to_string()));
    }
}
