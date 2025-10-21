use e2b::prelude::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new()?;

    println!("Creating sandbox with code interpreter...");
    let sandbox = client
        .sandbox()
        .template("code-interpreter-v1")
        .metadata(json!({"example": "basic"}))
        .timeout(300)
        .create()
        .await?;

    println!("Sandbox created: {}", sandbox.id());

    println!("\nRunning Python code...");
    let python_result = sandbox
        .run_python(
            r#"
print("Hello from Python in E2B!")
import sys
print(f"Python version: {sys.version}")

result = 2 + 2
print(f"2 + 2 = {result}")
    "#,
        )
        .await?;

    println!("Output:\n{}", python_result.stdout);

    println!("\nRunning JavaScript code...");
    let js_result = sandbox
        .run_javascript(
            r#"
console.log("Hello from JavaScript in E2B!");
console.log("Node version:", process.version);

const result = 2 + 2;
console.log(`2 + 2 = ${result}`);
    "#,
        )
        .await?;

    println!("Output:\n{}", js_result.stdout);

    println!("\nDeleting sandbox...");
    sandbox.delete().await?;
    println!("Done!");

    Ok(())
}
