use e2b::prelude::*;
use e2b::models::{WriteEntry, ReadFormat};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== E2B Filesystem Demo ===");

    let client = Client::new()?;

    println!("\n1. Creating sandbox...");
    let sandbox = client
        .sandbox()
        .template("base")
        .create()
        .await?;

    println!("Created sandbox: {}", sandbox.id());

    println!("\n2. Writing text files...");
    let write_info = sandbox.files().write_text("/tmp/hello.txt", "Hello, E2B!").await?;
    println!("Wrote file: {} ({} bytes)", write_info.path, write_info.size);

    println!("\n3. Reading text file...");
    let content = sandbox.files().read_text("/tmp/hello.txt").await?;
    println!("File content: {}", content);

    println!("\n4. Writing binary files...");
    let binary_data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
    let write_info = sandbox.files().write_binary("/tmp/hello.bin", binary_data).await?;
    println!("Wrote binary file: {} ({} bytes)", write_info.path, write_info.size);

    println!("\n5. Reading binary file...");
    let binary_content = sandbox.files().read_binary("/tmp/hello.bin").await?;
    println!("Binary content: {:?}", binary_content);

    println!("\n6. Writing multiple files at once...");
    let entries = vec![
        WriteEntry::text("/tmp/file1.txt", "Content of file 1"),
        WriteEntry::text("/tmp/file2.txt", "Content of file 2"),
        WriteEntry::binary("/tmp/file3.bin", vec![1, 2, 3, 4, 5]),
    ];

    let write_infos = sandbox.files().write_files(entries).await?;
    for info in write_infos {
        println!("Wrote: {} ({} bytes)", info.path, info.size);
    }

    println!("\n7. Listing directory contents...");
    let entries = sandbox.files().list("/tmp").await?;
    for entry in entries {
        let file_type = if entry.is_dir { "DIR" } else { "FILE" };
        println!("{}: {} ({} bytes)", file_type, entry.name, entry.size);
    }

    println!("\n8. Checking file existence...");
    let exists = sandbox.files().exists("/tmp/hello.txt").await?;
    println!("File exists: {}", exists);

    println!("\n9. Getting file information...");
    let file_info = sandbox.files().get_info("/tmp/hello.txt").await?;
    println!("File: {} (owner: {}, size: {} bytes)", file_info.name, file_info.owner, file_info.size);

    println!("\n10. Creating directories...");
    sandbox.files().make_dir("/tmp/test_dir").await?;
    println!("Created directory: /tmp/test_dir");

    println!("\n11. Moving/renaming files...");
    sandbox.files().rename("/tmp/hello.txt", "/tmp/test_dir/hello_moved.txt").await?;
    println!("Moved file to: /tmp/test_dir/hello_moved.txt");

    println!("\n12. Watching directory for changes...");
    let mut watch_handle = sandbox.files().watch_dir("/tmp/test_dir").await?;
    println!("Watching directory for changes...");

    // In a real scenario, you could listen for events:
    // tokio::spawn(async move {
    //     while let Some(event) = watch_handle.recv().await {
    //         println!("File event: {:?} on {}", event.event_type, event.path);
    //     }
    // });

    println!("\n13. Removing files...");
    sandbox.files().remove("/tmp/test_dir/hello_moved.txt").await?;
    println!("Removed file");

    println!("\n14. Cleaning up...");
    sandbox.delete().await?;
    println!("Sandbox deleted");

    println!("\n✅ Filesystem operations completed successfully!");

    Ok(())
}