use crate::error::{Error, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub debug: bool,
}

impl Config {
    pub fn new() -> Result<Self> {
        let api_key = env::var("E2B_API_KEY").map_err(|_| Error::ApiKeyNotFound)?;
        let debug = env::var("E2B_DEBUG")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        Ok(Self {
            api_key,
            base_url: if debug {
                "http://localhost:3000".to_string()
            } else {
                "https://api.e2b.app".to_string()
            },
            timeout_seconds: 300,
            max_retries: 3,
            debug,
        })
    }

    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        let debug = env::var("E2B_DEBUG")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        Self {
            api_key: api_key.into(),
            base_url: if debug {
                "http://localhost:3000".to_string()
            } else {
                "https://api.e2b.app".to_string()
            },
            timeout_seconds: 300,
            max_retries: 3,
            debug,
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

    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn is_debug(&self) -> bool {
        self.debug
    }

    pub fn sandbox_domain(&self) -> String {
        if self.debug {
            return "localhost".to_string();
        }

        let domain = env::var("E2B_SANDBOX_DOMAIN")
            .or_else(|_| env::var("E2B_DOMAIN"))
            .ok()
            .and_then(|d| {
                let trimmed = d.trim().trim_start_matches("api.").to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            });

        domain.unwrap_or_else(|| "e2b.dev".to_string())
    }
}
