use crate::{
    api::{SandboxApi, TemplateApi},
    config::Config,
    error::{Error, Result},
};
use reqwest::{header, Client as HttpClient};
use std::time::Duration;
use tracing::debug;

#[derive(Clone)]
pub struct Client {
    http: HttpClient,
    config: Config,
}

impl Client {
    pub fn new() -> Result<Self> {
        let config = Config::new()?;
        Self::with_config(config)
    }

    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        let config = Config::with_api_key(api_key);
        Self::with_config(config).expect("Failed to create client with provided API key")
    }

    pub fn with_config(config: Config) -> Result<Self> {
        let mut headers = header::HeaderMap::new();

        let api_key_header = header::HeaderValue::from_str(&config.api_key)
            .map_err(|_| Error::Configuration("Invalid API key format".to_string()))?;
        headers.insert("X-API-Key", api_key_header);
        headers.insert(header::USER_AGENT, header::HeaderValue::from_static("e2b-rust-sdk/0.1.0"));

        let http = HttpClient::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;

        debug!("E2B client initialized with base URL: {}", config.base_url);

        Ok(Self { http, config })
    }

    pub fn sandbox(&self) -> SandboxApi {
        SandboxApi::new(self.clone())
    }

    pub fn template(&self) -> TemplateApi {
        TemplateApi::new(self.clone())
    }

    pub(crate) fn http(&self) -> &HttpClient {
        &self.http
    }

    pub(crate) fn config(&self) -> &Config {
        &self.config
    }

    pub(crate) fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url, path)
    }
}