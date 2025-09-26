use e2b::models::WriteEntry;
use e2b::prelude::*;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    println!("=== E2B Filesystem Demo ===");

    let client = Client::new()?;

    println!("\n1. Creating sandbox...");
    let sandbox = client.sandbox().template("base").create().await?;

    println!("Created sandbox: {}", sandbox.id());
    let sandbox_info = sandbox.sandbox();
    println!(
        "Sandbox domain fields: domain={:?}, sandbox_domain={:?}",
        sandbox_info.domain, sandbox_info.sandbox_domain
    );

    println!("\n2. Writing text files...");
    let write_info = sandbox
        .files()
        .write_text("/tmp/hello.txt", "Hello, E2B!")
        .await?;
    println!("Wrote file: {}", write_info.path);
    assert_eq!(write_info.path, "/tmp/hello.txt");

    println!("\n3. Reading text file...");
    let content = sandbox.files().read_text("/tmp/hello.txt").await?;
    println!("File content: {}", content);
    assert_eq!(content, "Hello, E2B!");

    println!("\n4. Writing binary files...");
    let binary_data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
    let write_info = sandbox
        .files()
        .write_binary("/tmp/hello.bin", binary_data)
        .await?;
    println!("Wrote binary file: {}", write_info.path);
    assert_eq!(write_info.path, "/tmp/hello.bin");

    println!("\n5. Reading binary file...");
    let binary_content = sandbox.files().read_binary("/tmp/hello.bin").await?;
    println!("Binary content: {:?}", binary_content);
    assert_eq!(binary_content, vec![72, 101, 108, 108, 111]);

    println!("\n6. Writing multiple files at once...");
    let entries = vec![
        WriteEntry::text("/tmp/file1.txt", "Content of file 1"),
        WriteEntry::text("/tmp/file2.txt", "Content of file 2"),
        WriteEntry::binary("/tmp/file3.bin", vec![1, 2, 3, 4, 5]),
    ];

    let write_infos = sandbox.files().write_files(entries).await?;
    assert_eq!(write_infos.len(), 3);
    for info in &write_infos {
        println!("Wrote: {}", info.path);
    }
    assert!(write_infos
        .iter()
        .any(|info| info.path.ends_with("file1.txt")));

    println!("\n7. Listing directory contents...");
    let entries = sandbox.files().list("/tmp").await?;
    assert!(!entries.is_empty(), "expected /tmp to contain files");
    for entry in &entries {
        let file_type = if entry.is_dir { "DIR" } else { "FILE" };
        println!("{}: {} ({} bytes)", file_type, entry.name, entry.size);
    }
    assert!(entries.iter().any(|entry| entry.name == "hello.txt"));

    println!("\n8. Checking file existence...");
    let exists = sandbox.files().exists("/tmp/hello.txt").await?;
    println!("File exists: {}", exists);
    assert!(exists);

    println!("\n9. Getting file information...");
    let file_info = sandbox.files().get_info("/tmp/hello.txt").await?;
    println!(
        "File: {} (owner: {}, size: {} bytes)",
        file_info.name, file_info.owner, file_info.size
    );
    assert_eq!(file_info.name, "hello.txt");

    println!("\n10. Creating directories...");
    sandbox.files().make_dir("/tmp/test_dir").await?;
    println!("Created directory: /tmp/test_dir");

    println!("\n11. Moving/renaming files...");
    sandbox
        .files()
        .rename("/tmp/hello.txt", "/tmp/test_dir/hello_moved.txt")
        .await?;
    println!("Moved file to: /tmp/test_dir/hello_moved.txt");
    assert!(
        sandbox
            .files()
            .exists("/tmp/test_dir/hello_moved.txt")
            .await?
    );
    assert!(!sandbox.files().exists("/tmp/hello.txt").await?);

    println!("\n12. Watching directory for changes...");
    let _watch_handle = sandbox.files().watch_dir("/tmp/test_dir").await?;
    println!("Watching directory for changes...");

    // In a real scenario, you could listen for events:
    // tokio::spawn(async move {
    //     while let Some(event) = watch_handle.recv().await {
    //         println!("File event: {:?} on {}", event.event_type, event.path);
    //     }
    // });

    println!("\n13. Removing files...");
    sandbox
        .files()
        .remove("/tmp/test_dir/hello_moved.txt")
        .await?;
    println!("Removed file");
    assert!(
        !sandbox
            .files()
            .exists("/tmp/test_dir/hello_moved.txt")
            .await?
    );

    println!("\n14. Cleaning up...");
    sandbox.delete().await?;
    println!("Sandbox deleted");

    println!("\n✅ Filesystem operations completed successfully!");

    Ok(())
}
