use e2b::prelude::*;
use std::collections::HashMap;
use std::time::Duration;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    println!("=== E2B Commands Demo ===");
    println!("This example demonstrates command execution in E2B sandboxes.\n");

    let client = Client::new()?;

    println!("1. Creating sandbox...");
    let sandbox = client
        .sandbox()
        .template("base")
        .env_var("NODE_ENV", "development")
        .env_var("DEMO_MODE", "true")
        .timeout(600)
        .create()
        .await?;

    println!("✅ Created sandbox: {}\n", sandbox.id());

    println!("2. Running basic commands...");

    // Basic echo command
    let result = sandbox.commands().run("echo 'Hello from E2B Commands!'").await?;
    println!("Echo output: {}", result.stdout.trim());
    println!("Exit code: {}", result.exit_code);

    // System information commands
    let whoami_result = sandbox.commands().run("whoami").await?;
    println!("Current user: {}", whoami_result.stdout.trim());

    let pwd_result = sandbox.commands().run("pwd").await?;
    println!("Current directory: {}", pwd_result.stdout.trim());

    let uname_result = sandbox.commands().run("uname -a").await?;
    println!("System info: {}", uname_result.stdout.trim());

    println!("\n3. Environment variables test...");
    let env_result = sandbox.commands().run("echo \"NODE_ENV: $NODE_ENV, DEMO_MODE: $DEMO_MODE\"").await?;
    println!("Environment: {}", env_result.stdout.trim());

    println!("\n4. Running command with custom environment...");
    let mut envs = HashMap::new();
    envs.insert("CUSTOM_VAR".to_string(), "Hello from custom env!".to_string());
    envs.insert("TEST_NUMBER".to_string(), "42".to_string());

    let options = CommandOptions {
        envs: Some(envs),
        cwd: Some("/tmp".to_string()),
        timeout: Some(Duration::from_secs(30)),
        background: false,
    };

    let result = sandbox.commands().run_with_options("echo \"Custom: $CUSTOM_VAR, Number: $TEST_NUMBER, PWD: $PWD\"", &options).await?;
    println!("Custom env output: {}", result.stdout.trim());

    println!("\n5. File operations through commands...");

    // Create a test file
    let create_result = sandbox.commands().run("echo 'Hello World from command line!' > /tmp/test_file.txt").await?;
    if create_result.exit_code == 0 {
        println!("✅ File created successfully");
    }

    // Read the file
    let read_result = sandbox.commands().run("cat /tmp/test_file.txt").await?;
    println!("File content: {}", read_result.stdout.trim());

    // List directory
    let ls_result = sandbox.commands().run("ls -la /tmp/test_file.txt").await?;
    println!("File info: {}", ls_result.stdout.trim());

    println!("\n6. Running background command...");
    let handle = sandbox.commands().run_background("sleep 3 && echo 'Background task completed!' > /tmp/bg_result.txt").await?;
    println!("✅ Started background process with PID: {}", handle.pid());

    // Wait a moment then check if process is still running
    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("\n7. Listing running processes...");
    let processes = sandbox.commands().list().await?;
    println!("Found {} running processes:", processes.len());
    for process in &processes {
        println!("  PID: {}, Command: {}", process.pid, process.cmd);
    }

    // Wait for background task to complete
    println!("\n8. Waiting for background task...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check result
    let bg_result = sandbox.commands().run("cat /tmp/bg_result.txt 2>/dev/null || echo 'Background file not found'").await?;
    println!("Background task result: {}", bg_result.stdout.trim());

    println!("\n9. Interactive command demonstration...");
    // Start a simple process that will finish quickly to test background functionality
    // Let's try with 'echo' first to see if background processes work at all
    println!("Testing with echo first...");
    match sandbox.commands().run_background("echo 'Background process test'").await {
        Ok(handle) => {
            println!("✅ Started echo process with PID: {}", handle.pid());
        }
        Err(e) => {
            println!("❌ Failed to start echo process: {}", e);
        }
    }

    // Now try with cat
    println!("Testing with cat...");
    let cat_handle = sandbox.commands().run_background("cat").await?;
    println!("✅ Started interactive process (cat) with PID: {}", cat_handle.pid());

    // Send input to the process
    sandbox.commands().send_stdin(cat_handle.pid(), "Hello, World!").await?;
    println!("✅ Sent stdin input to process");

    // Wait a moment for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Try to read the output by connecting to the process
    let connect_handle = sandbox.commands().connect(cat_handle.pid()).await?;

    // Wait for the output with timeout
    match tokio::time::timeout(Duration::from_secs(2),
        sandbox.commands().wait_for_command(connect_handle)).await {
        Ok(Ok(result)) => {
            println!("Process output: '{}'", result.stdout);
        }
        Ok(Err(e)) => {
            println!("Error reading process output: {}", e);
        }
        Err(_) => {
            println!("Timeout - process still running");
            // Terminate the cat process
            sandbox.commands().kill(cat_handle.pid()).await?;
            println!("✅ Terminated interactive process");
        }
    }

    println!("\n10. Error handling demonstration...");
    let error_result = sandbox.commands().run("nonexistent_command").await?;
    println!("Error command exit code: {}", error_result.exit_code);
    println!("Error output: {}", error_result.stderr.trim());

    println!("\n11. Multiple commands in sequence...");
    let multi_cmd = r#"
        cd /tmp
        mkdir -p demo_dir
        cd demo_dir
        echo "File 1" > file1.txt
        echo "File 2" > file2.txt
        ls -la
        wc -l *.txt
        cd ..
        rm -rf demo_dir
        echo "Cleanup completed"
    "#;

    let multi_result = sandbox.commands().run(multi_cmd).await?;
    println!("Multi-command output:\n{}", multi_result.stdout);

    println!("\n12. Python command execution...");
    let python_cmd = r#"python3 -c "
import os
import sys
print(f'Python version: {sys.version.split()[0]}')
print(f'Current working directory: {os.getcwd()}')
print(f'Environment PATH length: {len(os.environ.get(\"PATH\", \"\").split(\":\"))}')
"#;

    let python_result = sandbox.commands().run(python_cmd).await?;
    println!("Python output:\n{}", python_result.stdout);

    println!("\n🧹 Cleanup...");
    sandbox.delete().await?;
    println!("✅ Sandbox deleted successfully!");

    Ok(())
}