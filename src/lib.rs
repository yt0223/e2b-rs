//! # E2B Rust SDK
//!
//! A Rust SDK for the E2B API that provides secure sandboxed code execution.
//!
//! ## Quick Start
//!
//! ```no_run
//! use e2b::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), e2b::Error> {
//!     let client = Client::new()?;
//!
//!     let sandbox = client
//!         .sandbox()
//!         .template("nodejs")
//!         .create()
//!         .await?;
//!
//!     let result = sandbox.run_code("console.log('Hello, E2B!')").await?;
//!     println!("Output: {}", result.stdout);
//!
//!     sandbox.delete().await?;
//!     Ok(())
//! }
//! ```

pub mod api;
pub mod client;
pub mod config;
pub mod error;
pub mod models;
pub mod rpc;

pub use client::Client;
pub use error::{Error, Result};

pub mod prelude {
    pub use crate::{Client, Error, Result};
    pub use crate::api::{CommandsApi, FilesystemApi, SandboxApi, TemplateApi};
    pub use crate::models::*;
}