use e2b::models::*;
use chrono::Utc;

#[tokio::test]
async fn test_write_entry_text() {
    let entry = WriteEntry::text("/tmp/test.txt", "Hello, World!");

    assert_eq!(entry.path, "/tmp/test.txt");
    match entry.data {
        WriteData::Text(content) => assert_eq!(content, "Hello, World!"),
        WriteData::Binary(_) => panic!("Expected text data"),
    }
}

#[tokio::test]
async fn test_write_entry_binary() {
    let data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
    let entry = WriteEntry::binary("/tmp/test.bin", data.clone());

    assert_eq!(entry.path, "/tmp/test.bin");
    match entry.data {
        WriteData::Binary(content) => assert_eq!(content, data),
        WriteData::Text(_) => panic!("Expected binary data"),
    }
}

#[tokio::test]
async fn test_entry_info_creation() {
    let entry = EntryInfo {
        path: "/tmp/test.txt".to_string(),
        name: "test.txt".to_string(),
        is_dir: false,
        size: 1024,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        permissions: "rw-r--r--".to_string(),
    };

    assert_eq!(entry.path, "/tmp/test.txt");
    assert_eq!(entry.name, "test.txt");
    assert!(!entry.is_dir);
    assert_eq!(entry.size, 1024);
    assert_eq!(entry.permissions, "rw-r--r--");
}

#[tokio::test]
async fn test_file_info_creation() {
    let file_info = FileInfo {
        path: "/tmp/test.txt".to_string(),
        name: "test.txt".to_string(),
        size: 2048,
        is_dir: false,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        permissions: 644,
        owner: "user".to_string(),
        group: "user".to_string(),
    };

    assert_eq!(file_info.path, "/tmp/test.txt");
    assert_eq!(file_info.name, "test.txt");
    assert_eq!(file_info.size, 2048);
    assert!(!file_info.is_dir);
    assert_eq!(file_info.permissions, 644);
    assert_eq!(file_info.owner, "user");
    assert_eq!(file_info.group, "user");
}

#[tokio::test]
async fn test_filesystem_event() {
    let event = FilesystemEvent {
        event_type: FilesystemEventType::Create,
        path: "/tmp/new_file.txt".to_string(),
        timestamp: Utc::now(),
        old_path: None,
    };

    match event.event_type {
        FilesystemEventType::Create => {},
        _ => panic!("Expected Create event type"),
    }

    assert_eq!(event.path, "/tmp/new_file.txt");
    assert!(event.old_path.is_none());
}

#[tokio::test]
async fn test_filesystem_event_move() {
    let event = FilesystemEvent {
        event_type: FilesystemEventType::Move,
        path: "/tmp/new_location.txt".to_string(),
        timestamp: Utc::now(),
        old_path: Some("/tmp/old_location.txt".to_string()),
    };

    match event.event_type {
        FilesystemEventType::Move => {},
        _ => panic!("Expected Move event type"),
    }

    assert_eq!(event.path, "/tmp/new_location.txt");
    assert_eq!(event.old_path, Some("/tmp/old_location.txt".to_string()));
}

#[tokio::test]
async fn test_read_result_text() {
    let result = ReadResult::Text("Hello, World!".to_string());

    match result {
        ReadResult::Text(content) => assert_eq!(content, "Hello, World!"),
        ReadResult::Binary(_) => panic!("Expected text result"),
    }
}

#[tokio::test]
async fn test_read_result_binary() {
    let data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
    let result = ReadResult::Binary(data.clone());

    match result {
        ReadResult::Binary(content) => assert_eq!(content, data),
        ReadResult::Text(_) => panic!("Expected binary result"),
    }
}