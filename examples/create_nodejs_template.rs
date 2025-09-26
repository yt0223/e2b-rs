use e2b::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new()?;

    println!("=== Creating Node.js Template ===");

    let dockerfile = r#"
FROM node:18-alpine

# Install common development tools
RUN apk add --no-cache \
    git \
    python3 \
    make \
    g++ \
    bash \
    curl \
    vim \
    nano

# Create app directory
WORKDIR /app

# Create a simple package.json
RUN echo '{"name": "e2b-nodejs", "version": "1.0.0", "main": "index.js"}' > package.json

# Install some common packages
RUN npm install express axios lodash

# Set up environment
ENV NODE_ENV=development
ENV PATH=$PATH:/app/node_modules/.bin

# Default command
CMD ["node", "--version"]
"#;

    match client
        .template()
        .name("nodejs-18")
        .description("Node.js 18 with common development tools and packages")
        .dockerfile(dockerfile.to_string())
        .create()
        .await
    {
        Ok(template_instance) => {
            println!("✅ Template created successfully!");
            println!("Template ID: {}", template_instance.id());
            println!("Name: {}", template_instance.template().name);
            println!(
                "Description: {}",
                template_instance
                    .template()
                    .description
                    .as_deref()
                    .unwrap_or("No description")
            );

            // Now trigger a build
            match template_instance.rebuild().await {
                Ok(build) => {
                    println!("🔨 Build started!");
                    println!("Build ID: {}", build.build_id);
                    println!("Status: {:?}", build.status);
                }
                Err(e) => {
                    println!("⚠️  Template created but build failed to start: {}", e);
                }
            }

            println!("\n📋 Next steps:");
            println!("1. Wait for the build to complete (check E2B dashboard)");
            println!(
                "2. Use template ID '{}' for Node.js specific environments",
                template_instance.id()
            );
            println!("3. Example usage:");
            println!(
                "   let sandbox = client.sandbox().template(\"{}\").create().await?;",
                template_instance.id()
            );
            println!("\n💡 Note: For multi-language code execution (Python + JavaScript), consider using:");
            println!("   let sandbox = client.sandbox().template(\"code-interpreter-v1\").create().await?;");

            println!("\n🔍 You can check the build status with:");
            println!("   cargo run --example list_templates");
        }
        Err(e) => {
            eprintln!("❌ Error creating template: {}", e);
            println!("\n🔧 Troubleshooting:");
            println!("1. Make sure your E2B_API_KEY is set correctly");
            println!("2. Check that you have permission to create templates");
            println!("3. Verify your account has available template slots");
        }
    }

    Ok(())
}
