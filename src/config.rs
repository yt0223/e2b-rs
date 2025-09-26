use crate::error::{Error, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

impl Config {
    pub fn new() -> Result<Self> {
        let api_key = env::var("E2B_API_KEY")
            .map_err(|_| Error::ApiKeyNotFound)?;

        Ok(Self {
            api_key,
            base_url: "https://api.e2b.app".to_string(),
            timeout_seconds: 300,
            max_retries: 3,
        })
    }

    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.e2b.app".to_string(),
            timeout_seconds: 300,
            max_retries: 3,
        }
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn timeout_seconds(mut self, timeout: u64) -> Self {
        self.timeout_seconds = timeout;
        self
    }

    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }
}