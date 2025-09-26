# E2B Rust SDK

A Rust SDK for the [E2B API](https://e2b.dev) that provides secure sandboxed code execution.

## Features

- 🦀 **Pure Rust** implementation with full async/await support
- 🛡️ **Type-safe** API with comprehensive error handling
- 🏗️ **Builder pattern** for intuitive sandbox and template configuration
- ⚡ **High performance** with connection pooling and automatic retries
- 📦 **Unix philosophy** - modular, composable, and simple to use
- 🔒 **Secure** API key authentication
- 🖥️ **Command execution** with real-time stdout/stderr streaming
- 📁 **File system operations** with read, write, watch, and batch support
- 🔄 **WebSocket/RPC** communication for real-time events

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
e2b = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use e2b::prelude::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize client (reads E2B_API_KEY from environment)
    let client = Client::new()?;

    // Create a sandbox with code interpreter support
    let sandbox = client
        .sandbox()
        .template("code-interpreter-v1")  // Use code interpreter template for multi-language support
        .metadata(json!({"project": "my-project"}))
        .create()
        .await?;

    // Execute Python code
    let python_result = sandbox.run_python("print('Hello from Python!'); 2 + 2").await?;
    println!("Python output: {}", python_result.stdout);

    // Execute JavaScript code
    let js_result = sandbox.run_javascript("console.log('Hello from JavaScript!'); console.log(2 + 2)").await?;
    println!("JavaScript output: {}", js_result.stdout);

    // Clean up
    sandbox.delete().await?;
    Ok(())
}
```

## Authentication

Set your E2B API key as an environment variable:

```bash
export E2B_API_KEY=e2b_your_api_key_here
```

Or provide it directly when creating the client:

```rust
let client = Client::with_api_key("e2b_your_api_key_here");
```

Get your API key from the [E2B Dashboard](https://e2b.dev/dashboard?tab=keys).

## Code Execution

The E2B Rust SDK supports multi-language code execution through the code interpreter template. This template provides a Jupyter-based environment that supports multiple programming languages.

### Template Requirements

For multi-language code execution, use the `code-interpreter-v1` template:

```rust
let sandbox = client
    .sandbox()
    .template("code-interpreter-v1")  // Required for multi-language support
    .create()
    .await?;
```

**Important**: The `code-interpreter-v1` template is required for language-specific code execution. Other templates may only support basic code execution without language selection.

### Language-Specific Code Execution

```rust
// Execute Python code
let python_result = sandbox.run_python(r#"
import numpy as np
import matplotlib.pyplot as plt

# Create some data
x = np.linspace(0, 10, 100)
y = np.sin(x)

print(f"Generated {len(x)} data points")
print(f"Max value: {np.max(y):.3f}")
"#).await?;

// Execute JavaScript/Node.js code
let js_result = sandbox.run_javascript(r#"
const fs = require('fs');
const data = { message: "Hello from JavaScript!", timestamp: Date.now() };

console.log("Data:", JSON.stringify(data, null, 2));
console.log("Node version:", process.version);
"#).await?;

// Generic code execution with language parameter
let result = sandbox.run_code_with_language("print('Hello, World!')", "python").await?;
```

### Code Execution Results

Code execution returns rich result objects with stdout, stderr, results, and error information:

```rust
let result = sandbox.run_python("print('Hello'); raise ValueError('test error')").await?;

println!("Standard output: {}", result.stdout);
println!("Standard error: {}", result.stderr);

if let Some(error) = result.error {
    println!("Execution error: {}", error.name);
    println!("Error message: {}", error.value);
    println!("Traceback: {}", error.traceback);
}

// Display data results (plots, images, etc.)
for display_result in result.results {
    println!("Result type: {}", display_result.result_type);
    // Access display data (HTML, images, etc.)
}
```

## Usage Examples

### Creating Sandboxes with Configuration

```rust
use std::collections::HashMap;
use std::time::Duration;

let mut env_vars = HashMap::new();
env_vars.insert("NODE_ENV".to_string(), "production".to_string());

let sandbox = client
    .sandbox()
    .template("nodejs")
    .env_vars(env_vars)
    .cpu_count(4)
    .memory_mb(2048)
    .timeout(Duration::from_secs(300))
    .metadata(json!({
        "project": "my-app",
        "version": "1.0.0"
    }))
    .create()
    .await?;
```

### Running Code with Timeout

```rust
use std::time::Duration;

let result = sandbox
    .run_code_with_timeout("console.log('Fast execution')", Duration::from_secs(5))
    .await?;
```

### Managing Sandbox Lifecycle

```rust
// List all sandboxes
let sandboxes = client.sandbox().list().await?;

// Pause and resume
sandbox.pause().await?;
sandbox.resume().await?;

// Get metrics and logs
let metrics = sandbox.metrics().await?;
let logs = sandbox.logs().await?;

// Clean up
sandbox.delete().await?;
```

### Working with Templates

```rust
// List available templates
let templates = client.template().list().await?;

// Create a new template
let template = client
    .template()
    .name("my-custom-template")
    .description("A custom Node.js environment")
    .dockerfile(r#"
FROM node:18-alpine
RUN npm install -g typescript
COPY . /app
WORKDIR /app
    "#)
    .create()
    .await?;

// Rebuild template
let build = template.rebuild().await?;
```

## Command Execution

Execute commands in the sandbox with full control over environment, working directory, and I/O:

```rust
// Run a simple command
let result = sandbox.commands().run("echo 'Hello, World!'").await?;
println!("Output: {}", result.stdout);
println!("Exit code: {}", result.exit_code);

// Run command with custom environment and working directory
let mut envs = HashMap::new();
envs.insert("NODE_ENV".to_string(), "production".to_string());

let options = CommandOptions {
    envs: Some(envs),
    cwd: Some("/app".to_string()),
    timeout: Some(Duration::from_secs(30)),
    background: false,
};

let result = sandbox.commands().run_with_options("npm start", &options).await?;

// Run commands in background
let handle = sandbox.commands().run_background("long-running-task").await?;
println!("Started process with PID: {}", handle.pid());

// List running processes
let processes = sandbox.commands().list().await?;
for process in processes {
    println!("PID: {}, Command: {}", process.pid, process.cmd);
}

// Send input to stdin
sandbox.commands().send_stdin(handle.pid(), "input data\n").await?;

// Kill a process
sandbox.commands().kill(handle.pid()).await?;
```

## File System Operations

Comprehensive file system operations with support for text, binary, and batch operations:

```rust
// Write and read text files
let write_info = sandbox.files().write_text("/tmp/hello.txt", "Hello, E2B!").await?;
let content = sandbox.files().read_text("/tmp/hello.txt").await?;

// Write and read binary files
let binary_data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f];
sandbox.files().write_binary("/tmp/data.bin", binary_data).await?;
let binary_content = sandbox.files().read_binary("/tmp/data.bin").await?;

// Batch file operations
let entries = vec![
    WriteEntry::text("/tmp/file1.txt", "Content 1"),
    WriteEntry::text("/tmp/file2.txt", "Content 2"),
    WriteEntry::binary("/tmp/file3.bin", vec![1, 2, 3, 4, 5]),
];
let write_infos = sandbox.files().write_files(entries).await?;

// List directory contents
let entries = sandbox.files().list("/tmp").await?;
for entry in entries {
    let file_type = if entry.is_dir { "DIR" } else { "FILE" };
    println!("{}: {} ({} bytes)", file_type, entry.name, entry.size);
}

// File operations
let exists = sandbox.files().exists("/tmp/hello.txt").await?;
let file_info = sandbox.files().get_info("/tmp/hello.txt").await?;

// Directory operations
sandbox.files().make_dir("/tmp/new_directory").await?;
sandbox.files().rename("/tmp/old.txt", "/tmp/new.txt").await?;
sandbox.files().remove("/tmp/unwanted.txt").await?;

// Watch directory for changes
let mut watch_handle = sandbox.files().watch_dir("/tmp").await?;
tokio::spawn(async move {
    while let Some(event) = watch_handle.recv().await {
        println!("File event: {:?} on {}", event.event_type, event.path);
    }
});
```

## Error Handling

The SDK provides comprehensive error handling:

```rust
use e2b::{Error, Result};

match sandbox.run_code("invalid syntax").await {
    Ok(result) => println!("Success: {}", result.stdout),
    Err(Error::Api { status, message }) => {
        eprintln!("API Error {}: {}", status, message);
    }
    Err(Error::Timeout) => {
        eprintln!("Code execution timed out");
    }
    Err(Error::NotFound(resource)) => {
        eprintln!("Resource not found: {}", resource);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Configuration

You can customize the client configuration:

```rust
use e2b::Client;

let client = Client::with_api_key("your_api_key")
    .base_url("https://api.e2b.dev")
    .timeout_seconds(30)
    .max_retries(5)
    .build()?;
```

## Examples

See the [examples](examples/) directory for more comprehensive usage examples:

- [`basic.rs`](examples/basic.rs) - Simple multi-language code execution
- [`code_interpreter.rs`](examples/code_interpreter.rs) - Comprehensive code interpreter demonstration with Python and JavaScript
- [`sandbox.rs`](examples/sandbox.rs) - Advanced sandbox management features
- [`commands_demo.rs`](examples/commands_demo.rs) - Command execution API demonstration
- [`filesystem_demo.rs`](examples/filesystem_demo.rs) - File system operations API demonstration
- [`create_nodejs_template.rs`](examples/create_nodejs_template.rs) - Custom template creation
- [`list_templates.rs`](examples/list_templates.rs) - Template management

Run examples with:

```bash
cargo run --example basic
cargo run --example code_interpreter
cargo run --example sandbox
cargo run --example commands_demo
cargo run --example filesystem_demo
cargo run --example create_nodejs_template
cargo run --example list_templates
```

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Support

- [E2B Documentation](https://e2b.dev/docs)
- [API Reference](https://docs.e2b.dev)
- [GitHub Issues](https://github.com/e2b-dev/e2b-rs/issues)