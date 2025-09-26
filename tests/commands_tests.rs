use e2b::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

#[tokio::test]
async fn test_commands_api_creation() {
    let client = Client::with_api_key("test_key");
    // Just test that client was created successfully without API key validation
}

#[tokio::test]
async fn test_command_options_default() {
    use e2b::models::CommandOptions;
    let options = CommandOptions::default();

    assert_eq!(options.background, false);
    assert_eq!(options.timeout, Some(Duration::from_secs(60)));
    assert!(options.envs.is_none());
    assert!(options.cwd.is_none());
}

#[tokio::test]
async fn test_command_options_with_env() {
    use e2b::models::CommandOptions;
    let mut envs = HashMap::new();
    envs.insert("TEST_VAR".to_string(), "test_value".to_string());

    let options = CommandOptions {
        envs: Some(envs),
        cwd: Some("/tmp".to_string()),
        timeout: Some(Duration::from_secs(30)),
        background: true,
    };

    assert_eq!(options.background, true);
    assert_eq!(options.timeout, Some(Duration::from_secs(30)));
    assert!(options.envs.is_some());
    assert_eq!(options.cwd, Some("/tmp".to_string()));
}

#[tokio::test]
async fn test_process_info_creation() {
    use e2b::models::ProcessInfo;

    let mut envs = HashMap::new();
    envs.insert("PATH".to_string(), "/usr/bin".to_string());

    let process = ProcessInfo {
        pid: 1234,
        tag: Some("test".to_string()),
        cmd: "echo".to_string(),
        args: vec!["hello".to_string()],
        envs,
        cwd: Some("/home/user".to_string()),
    };

    assert_eq!(process.pid, 1234);
    assert_eq!(process.tag, Some("test".to_string()));
    assert_eq!(process.cmd, "echo");
    assert_eq!(process.args, vec!["hello"]);
    assert_eq!(process.cwd, Some("/home/user".to_string()));
}

#[tokio::test]
async fn test_command_result_creation() {
    use e2b::models::CommandResult;

    let result = CommandResult {
        stdout: "Hello, World!".to_string(),
        stderr: "".to_string(),
        exit_code: 0,
        execution_time: Some(Duration::from_millis(100)),
    };

    assert_eq!(result.stdout, "Hello, World!");
    assert_eq!(result.stderr, "");
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.execution_time, Some(Duration::from_millis(100)));
}