use grinta::core::{CommandItem, Handler, CommandType};
use grinta::history;
use grinta::state::AppState;
use tempfile::TempDir;
use std::env;

#[tokio::test]
async fn test_full_application_workflow() {
    // Set up temporary environment
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();
    env::set_var("HOME", temp_path);

    // Create test items
    let mut app_item = CommandItem::new("Test App", Handler::App, "/Applications/Test.app");
    app_item.kind = CommandType::App;
    
    let mut note_item = CommandItem::new("Test Note", Handler::Note, "note-123");
    note_item.kind = CommandType::Note;
    
    let items = vec![app_item.clone(), note_item.clone()];

    // Test AppState creation and filtering
    let mut state = AppState::new(vec![], items);
    
    // Test empty query (should show empty since no history)
    state.filter_items();
    assert!(state.filtered_items.is_empty());
    
    // Test with query
    state.query.insert_str("test");
    state.filter_items();
    assert_eq!(state.filtered_items.len(), 2);
    
    // Test selection
    state.table_state.select(Some(0));
    let selected = state.get_selected_item();
    assert!(selected.is_some());
    
    // Test history functionality
    let mut history = vec![];
    let result = history::add_to_history(&mut history, app_item.clone());
    assert!(result.is_ok());
    assert_eq!(history.len(), 1);
    assert!(history[0].ran_at.is_some());
    
    // Test history persistence
    let save_result = history::save_history(&history);
    assert!(save_result.is_ok());
    
    let loaded_history = history::load_history().unwrap();
    assert_eq!(loaded_history.len(), 1);
    assert_eq!(loaded_history[0].label, "Test App");
}

#[tokio::test]
async fn test_fuzzy_matching_integration() {
    let items = vec![
        CommandItem::new("Cursor Editor", Handler::App, "cursor"),
        CommandItem::new("Calculator", Handler::App, "calc"),
        CommandItem::new("Chrome Browser", Handler::App, "chrome"),
        CommandItem::new("Code Editor", Handler::App, "code"),
    ];
    
    let mut state = AppState::new(vec![], items);
    
    // Test fuzzy matching with "cur"
    state.query.insert_str("cur");
    state.filter_items();
    
    // Should match "Cursor Editor" best
    assert!(!state.filtered_items.is_empty());
    
    // Test case insensitivity
    state.query.delete_line_by_end();
    state.query.delete_line_by_head();
    state.query.insert_str("CUR");
    state.filter_items();
    
    assert!(!state.filtered_items.is_empty());
}

#[tokio::test]
async fn test_error_handling_integration() {
    let mut state = AppState::new(vec![], vec![]);
    
    // Test error setting and clearing
    assert!(state.error_message.is_none());
    
    state.set_error("Test error message".to_string());
    assert_eq!(state.error_message, Some("Test error message".to_string()));
    
    state.clear_error();
    assert!(state.error_message.is_none());
}

#[tokio::test]
async fn test_mixed_data_sources() {
    let app_items = vec![
        CommandItem::new("App 1", Handler::App, "app1"),
        CommandItem::new("App 2", Handler::App, "app2"),
    ];
    
    let fs_items = vec![
        CommandItem::new("file1.txt", Handler::File, "/path/file1.txt"),
        CommandItem::new("folder1", Handler::Folder, "/path/folder1"),
    ];
    
    let mut web_item = CommandItem::new("web search", Handler::Url, "https://example.com");
    web_item.kind = CommandType::WebSuggestion;
    let web_items = vec![web_item];
    
    let mut state = AppState::new(vec![], app_items);
    state.fs_items = fs_items;
    state.web_items = web_items;
    
    // Test that all sources are combined
    state.query.insert_str("1");
    state.filter_items();
    
    // Should include items from all sources that match
    let labels: Vec<&str> = state.filtered_items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"App 1"));
    assert!(labels.contains(&"file1.txt"));
    assert!(labels.contains(&"folder1"));
}

#[test]
fn test_handler_icon_consistency() {
    // Test that all handlers have consistent icons and strings
    let handlers = vec![
        Handler::App,
        Handler::Note,
        Handler::Url,
        Handler::File,
        Handler::Folder,
        Handler::Automation,
    ];
    
    for handler in handlers {
        let icon = handler.to_icon();
        let string = handler.to_string();
        
        assert!(!icon.is_empty());
        assert!(!string.is_empty());
        
        // Test that creating an item works
        let item = CommandItem::new("Test", handler, "test");
        assert_eq!(item.handler, handler);
        assert_eq!(item.icon, icon);
    }
}

#[tokio::test]
async fn test_concurrent_operations() {
    // Test that multiple operations can run concurrently without issues
    let tasks = (0..10).map(|i| {
        tokio::spawn(async move {
            let item = CommandItem::new(&format!("Item {}", i), Handler::App, &format!("app{}", i));
            item.label.len() // Simple operation
        })
    }).collect::<Vec<_>>();
    
    let results = futures::future::join_all(tasks).await;
    
    // All tasks should complete successfully
    assert_eq!(results.len(), 10);
    for result in results {
        assert!(result.is_ok());
        assert!(result.unwrap() > 0);
    }
}

#[test]
fn test_command_type_variants() {
    // Test all command type variants
    let types = vec![
        CommandType::App,
        CommandType::Bookmark,
        CommandType::Note,
        CommandType::WebSearch,
        CommandType::WebSuggestion,
    ];
    
    for cmd_type in types {
        // Each type should be distinct from default (Unknown)
        assert_ne!(cmd_type, CommandType::default());
    }
    
    // Test default
    assert_eq!(CommandType::default(), CommandType::Unknown);
    
    // Test that Unknown equals default
    assert_eq!(CommandType::Unknown, CommandType::default());
} 