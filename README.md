# E2B Rust SDK

Unofficial Rust bindings for the [E2B](https://e2b.dev) sandbox platform. The crate exposes a lightweight `Client` that lets you create sandboxes, run commands, interact with the filesystem, and work with the code-interpreter template from async Rust code.

> **Status:** experimental. The API is evolving quickly and may change without notice.

## Features

- Async-first interface built on `tokio`
- Sandbox lifecycle helpers (`sandbox().template(...).create().await?`)
- Command execution with background jobs, stdin streaming and process listing
- Filesystem helpers that read, write, rename and watch files via envd RPC
- Code interpreter helper for the `code-interpreter-v1` template (multi-language execution)
- Metrics & log accessors with debug logging of raw payloads for easier troubleshooting

## Installation

```toml
[dependencies]
e2b = "0.1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1" # optional but handy for debugging
```

## Getting Started

1. Generate an API key in the [E2B dashboard](https://e2b.dev/dashboard?tab=keys).
2. Export it before running the examples or your own code:

   ```bash
   export E2B_API_KEY=e2b_xxx
   ```

3. Run any of the bundled examples:

   ```bash
   cargo run --example sandbox          # lifecycle + metrics/logs demo
   cargo run --example commands_demo    # process management
   cargo run --example filesystem_demo  # file helpers
   cargo run --example code_interpreter # multi-language execution
   ```

   Set `RUST_LOG=debug` to see the raw HTTP/RPC payloads that helped troubleshoot integrations:

   ```bash
   RUST_LOG=debug cargo run --example sandbox
   ```

## High-level API

```rust
use e2b::prelude::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new()?; // reads E2B_API_KEY from the environment

    let sandbox = client
        .sandbox()
        .template("base")
        .metadata(json!({"demo": "rust"}))
        .create()
        .await?;

    let result = sandbox.commands().run("echo hello").await?;
    println!("stdout: {}", result.stdout.trim());

    sandbox.delete().await?;
    Ok(())
}
```

### Commands

```rust
let handle = sandbox.commands().run_background("sleep 3 && echo done > /tmp/out").await?;
tracing::info!(pid = handle.pid(), "background command started");

let running = sandbox.commands().list().await?;
for proc in running {
    println!("pid={} cmd={} args={:?}", proc.pid, proc.cmd, proc.args);
}
```

### Filesystem

```rust
sandbox.files().write_text("/tmp/hello.txt", "hi from rust").await?;
let body = sandbox.files().read_text("/tmp/hello.txt").await?;
println!("{}", body);

sandbox.files().rename("/tmp/hello.txt", "/tmp/archive/hello.txt").await?;
```

### Code Interpreter

```rust
let py = sandbox.run_python("print('hello from python')").await?;
println!("python stdout: {}", py.stdout);

let js = sandbox.run_javascript("console.log('hello from javascript')").await?;
println!("node stdout: {}", js.stdout);
```

### Metrics & Logs

Metrics are emitted by envd at a low cadence. Immediately after creating a sandbox you may receive an empty array. The SDK surfaces this as `SandboxMetrics::default()`.

```rust
match sandbox.metrics().await {
    Ok(metrics) => {
        println!(
            "cpu cores={} cpu_used={:.2}% mem={:.1}/{:.1} MB",
            metrics.cpu_count,
            metrics.cpu_used_pct,
            metrics.mem_used as f64 / 1_048_576.0,
            metrics.mem_total as f64 / 1_048_576.0,
        );
    }
    Err(err) => tracing::warn!(?err, "failed to load metrics"),
}

for entry in sandbox.logs().await?.into_iter().take(5) {
    println!("[{entry:?}]");
}
```

## Environment Tweaks

- `E2B_SANDBOX_DOMAIN` – override the default `*.e2b.dev` envd domain if you run a custom deployment.

## Development Notes

- Examples intentionally dump warnings instead of aborting whenever the remote payload differs from the expected format. Investigate the logs and adjust assertions to match your envd build when necessary.
- `cargo check --examples` validates all demos without running the remote calls.
- A full `cargo test` is currently disabled because the public CI environment does not provide API credentials.

## License

MIT
