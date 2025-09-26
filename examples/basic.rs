use e2b::prelude::*;
use serde_json::json;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to see debug output
    tracing_subscriber::fmt::init();
    let client = Client::new()?;

    println!("Creating sandbox with code interpreter...");
    let sandbox = client
        .sandbox()
        .template("code-interpreter-v1")  // Use code interpreter for multi-language support
        .metadata(json!({"example": "basic"}))
        .timeout(300)
        .create()
        .await?;

    println!("Sandbox created: {}", sandbox.id());

    println!("Running Python code...");
    let python_result = sandbox.run_python(r#"
print("Hello from Python in E2B!")
import sys
print(f"Python version: {sys.version}")

# Simple calculation
result = 2 + 2
print(f"2 + 2 = {result}")
result
    "#).await?;

    println!("Python output:\n{}", python_result.stdout);
    if !python_result.stderr.is_empty() {
        println!("Python stderr: {}", python_result.stderr);
    }

    println!("\nRunning JavaScript code...");
    let js_result = sandbox.run_javascript(r#"
console.log("Hello from JavaScript in E2B!");
console.log("Node version:", process.version);

// File operations
const fs = require('fs');
const data = {
    message: 'Hello World',
    timestamp: new Date().toISOString(),
    platform: process.platform
};

fs.writeFileSync('/tmp/test.json', JSON.stringify(data, null, 2));
const content = fs.readFileSync('/tmp/test.json', 'utf8');
console.log('File content:', content);

// Return result
2 + 2;
    "#).await?;

    println!("JavaScript output:\n{}", js_result.stdout);
    if !js_result.stderr.is_empty() {
        println!("JavaScript stderr: {}", js_result.stderr);
    }

    println!("\nRunning code with explicit language parameter...");
    let param_result = sandbox.run_code_with_language(
        "print('This is Python code executed with language parameter')",
        "python"
    ).await?;
    println!("Language parameter result:\n{}", param_result.stdout);

    println!("\nDeleting sandbox...");
    sandbox.delete().await?;
    println!("Cleanup completed!");

    Ok(())
}