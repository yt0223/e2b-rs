use e2b::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_client_creation() {
    let _client = Client::with_api_key("test_key");
    // Just test that client was created successfully
}

#[tokio::test]
async fn test_client_with_env_var() {
    std::env::set_var("E2B_API_KEY", "test_env_key");
    let result = Client::new();
    std::env::remove_var("E2B_API_KEY");

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_client_without_env_var() {
    std::env::remove_var("E2B_API_KEY");
    let result = Client::new();

    match result {
        Err(Error::ApiKeyNotFound) => (),
        _ => panic!("Expected ApiKeyNotFound error"),
    }
}

#[tokio::test]
async fn test_sandbox_builder() {
    let client = Client::with_api_key("test_key");

    let _builder = client
        .sandbox()
        .template("nodejs")
        .metadata(json!({"test": true}))
        .timeout(300);

    // Test that builder pattern works without accessing private fields
}

#[tokio::test]
async fn test_template_builder() {
    let client = Client::with_api_key("test_key");

    let _builder = client
        .template()
        .name("test-template");

    // Test that template builder pattern works
}

#[tokio::test]
async fn test_client_apis() {
    let client = Client::with_api_key("test_key");

    // Test that we can access the API endpoints
    let _sandbox_api = client.sandbox();
    let _template_api = client.template();
}

#[tokio::test]
async fn test_error_types() {
    let api_error = Error::Api {
        status: 404,
        message: "Not found".to_string(),
    };

    match api_error {
        Error::Api { status, message } => {
            assert_eq!(status, 404);
            assert_eq!(message, "Not found");
        }
        _ => panic!("Expected API error"),
    }

    let timeout_error = Error::Timeout;
    matches!(timeout_error, Error::Timeout);

    let not_found_error = Error::NotFound("test".to_string());
    matches!(not_found_error, Error::NotFound(_));
}