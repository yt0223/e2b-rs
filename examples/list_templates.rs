use e2b::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new()?;

    println!("=== Available Templates ===");

    match client.template().list().await {
        Ok(templates) => {
            if templates.is_empty() {
                println!("No templates found. You need to create a template first.");
                println!("\nTo create a template:");
                println!("1. Go to https://e2b.dev/dashboard");
                println!("2. Click 'Templates' in the sidebar");
                println!("3. Click 'Create template'");
                println!("4. Choose a base image or upload a Dockerfile");
                println!("5. Give it a name and build it");
            } else {
                for template in templates {
                    println!("Template ID: {}", template.template_id);
                    println!("  Name: {}", template.name);
                    println!("  Description: {}", template.description.as_deref().unwrap_or("No description"));
                    println!("  Status: {}", if template.public { "Public" } else { "Private" });
                    println!("  CPU: {} cores, Memory: {} MB, Disk: {} MB",
                            template.cpu_count, template.memory_mb, template.disk_mb);
                    println!("  Created: {}", template.created_at);
                    println!("---");
                }
            }
        }
        Err(e) => {
            eprintln!("Error fetching templates: {}", e);
            println!("\nMake sure you have:");
            println!("1. Set your E2B_API_KEY environment variable");
            println!("2. Created at least one template in the E2B dashboard");
        }
    }

    Ok(())
}