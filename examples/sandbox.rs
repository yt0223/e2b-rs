use e2b::prelude::*;
use serde_json::json;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new()?;

    println!("Listing all sandboxes...");
    let sandboxes = client.sandbox().list().await?;
    println!("Found {} existing sandboxes", sandboxes.len());
    for sb in &sandboxes {
        println!("- {}: {} ({})", sb.sandbox_id, sb.template_id, if sb.is_live { "live" } else { "stopped" });
    }

    println!("\nCreating new sandbox with environment variables...");
    let mut env_vars = std::collections::HashMap::new();
    env_vars.insert("NODE_ENV".to_string(), "development".to_string());
    env_vars.insert("DEBUG".to_string(), "1".to_string());

    let sandbox = client
        .sandbox()
        .template("code-interpreter-v1")  // Use the code interpreter template
        .metadata(json!({
            "project": "e2b-rust-sdk-demo",
            "created_by": "rust-example"
        }))
        .env_vars(env_vars)
        .timeout(600) // 600 seconds
        .allow_internet_access(true)
        .create()
        .await?;

    println!("Sandbox created: {}", sandbox.id());
    println!("Template ID: {}", sandbox.sandbox().template_id);
    println!("CPU Count: {}", sandbox.sandbox().cpu_count);
    println!("Memory: {} MB", sandbox.sandbox().memory_mb);

    println!("\nTesting Python code execution...");
    let python_result = sandbox.run_python("print('Hello from Python!'); import sys; print(f'Python version: {sys.version}')").await?;
    println!("Python output:\n{}", python_result.stdout);
    if !python_result.stderr.is_empty() {
        println!("Python stderr:\n{}", python_result.stderr);
    }

    println!("\nTesting JavaScript code execution...");
    let js_result = sandbox.run_javascript("console.log('Hello from JavaScript!'); console.log('Node version:', process.version)").await?;
    println!("JavaScript output:\n{}", js_result.stdout);
    if !js_result.stderr.is_empty() {
        println!("JavaScript stderr:\n{}", js_result.stderr);
    }

    println!("\nTesting environment variables with JavaScript...");
    let env_result = sandbox.run_javascript("console.log('NODE_ENV:', process.env.NODE_ENV); console.log('DEBUG:', process.env.DEBUG)").await?;
    println!("Environment test output:\n{}", env_result.stdout);

    println!("\nTesting error handling with Python...");
    let error_result = sandbox.run_python("raise Exception('Test error from Python'); print('This should not run')").await?;
    if let Some(error) = &error_result.error {
        println!("Python error - Name: {}", error.name);
        println!("Python error - Value: {}", error.value);
    }

    println!("\nTesting long-running code with timeout...");
    match sandbox.run_code_with_timeout("setTimeout(() => console.log('Done!'), 100); 'Started'", Duration::from_millis(50)).await {
        Ok(result) => println!("Unexpected success: {}", result.stdout),
        Err(Error::Timeout) => println!("Correctly timed out as expected"),
        Err(e) => println!("Unexpected error: {}", e),
    }

    println!("\nGetting sandbox metrics...");
    match sandbox.metrics().await {
        Ok(metrics) => {
            println!("CPU Usage: {:.2}%", metrics.cpu_usage_percent);
            println!("Memory Usage: {} MB / {} MB", metrics.memory_usage_mb, metrics.memory_limit_mb);
            println!("Disk Usage: {} MB / {} MB", metrics.disk_usage_mb, metrics.disk_limit_mb);
        }
        Err(e) => println!("Failed to get metrics: {}", e),
    }

    println!("\nGetting sandbox logs...");
    match sandbox.logs().await {
        Ok(logs) => {
            println!("Found {} log entries", logs.len());
            for (i, log) in logs.iter().take(5).enumerate() {
                println!("  {}: [{:?}] {} - {}", i + 1, log.level, log.source, log.message);
            }
            if logs.len() > 5 {
                println!("  ... and {} more", logs.len() - 5);
            }
        }
        Err(e) => println!("Failed to get logs: {}", e),
    }

    println!("\nTesting pause and resume...");
    sandbox.pause().await?;
    println!("Sandbox paused");

    tokio::time::sleep(Duration::from_secs(2)).await;

    sandbox.resume().await?;
    println!("Sandbox resumed");

    println!("\nTesting file operations with Python...");
    let python_file_result = sandbox.run_python(r#"
import os
import json
from datetime import datetime

test_dir = '/tmp/e2b-test-python'
test_file = os.path.join(test_dir, 'data.json')

os.makedirs(test_dir, exist_ok=True)

data = {
    'timestamp': datetime.now().isoformat(),
    'pid': os.getpid(),
    'python_version': os.sys.version.split()[0]
}

with open(test_file, 'w') as f:
    json.dump(data, f, indent=2)

print(f'Created file: {test_file}')

with open(test_file, 'r') as f:
    content = f.read()
    print(f'File contents:\n{content}')

file_size = os.path.getsize(test_file)
print(f'File size: {file_size} bytes')
    "#).await?;

    println!("Python file operations result:\n{}", python_file_result.stdout);

    println!("\nTesting file operations with JavaScript...");
    let js_file_result = sandbox.run_javascript(r#"
const fs = require('fs');
const path = require('path');

const testDir = '/tmp/e2b-test-js';
const testFile = path.join(testDir, 'data.json');

if (!fs.existsSync(testDir)) {
    fs.mkdirSync(testDir, { recursive: true });
}

const data = {
    timestamp: new Date().toISOString(),
    pid: process.pid,
    platform: process.platform,
    nodeVersion: process.version
};

fs.writeFileSync(testFile, JSON.stringify(data, null, 2));
console.log('Created file:', testFile);

const content = fs.readFileSync(testFile, 'utf8');
console.log('File contents:\\n', content);

const stats = fs.statSync(testFile);
console.log('File size:', stats.size, 'bytes');
    "#).await?;

    println!("JavaScript file operations result:\n{}", js_file_result.stdout);

    println!("\nCleaning up...");
    sandbox.delete().await?;
    println!("Sandbox deleted successfully!");

    Ok(())
}