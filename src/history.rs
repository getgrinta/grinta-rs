use crate::core::CommandItem;
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

const HISTORY_FILE: &str = "grinta_history.json";

fn history_file_path() -> Result<PathBuf> {
    let mut path = dirs::data_dir().context("Failed to get data directory")?;
    path.push("grinta-rs");
    fs::create_dir_all(&path)?;
    path.push(HISTORY_FILE);
    Ok(path)
}

pub fn load_history() -> Result<Vec<CommandItem>> {
    let path = history_file_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let history = serde_json::from_str(&contents).unwrap_or_else(|_| Vec::new());
    Ok(history)
}

pub fn save_history(history: &[CommandItem]) -> Result<()> {
    let path = history_file_path()?;
    let mut file = File::create(path)?;
    let json = serde_json::to_string_pretty(history)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

pub fn add_to_history(history: &mut Vec<CommandItem>, mut item: CommandItem) -> Result<()> {
    item.mark_executed();
    
    history.retain(|h| h.label != item.label || h.handler != item.handler || h.value != item.value);
    history.push(item);

    save_history(history)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Handler;
    use tempfile::TempDir;
    use std::env;

    fn create_test_item(label: &str, handler: Handler, value: &str) -> CommandItem {
        CommandItem::new(label, handler, value)
    }

    #[test]
    fn test_load_history_empty() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        
        // Set temporary data dir
        env::set_var("HOME", temp_path);
        
        let result = load_history();
        assert!(result.is_ok());
        let history = result.unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_save_and_load_history() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        env::set_var("HOME", temp_path);

        let mut items = vec![
            create_test_item("Test App", Handler::App, "/Applications/Test.app"),
            create_test_item("Test Note", Handler::Note, "note-123"),
            create_test_item("Test File", Handler::File, "/path/to/file.txt"),
        ];

        // Mark one as executed
        items[0].mark_executed();

        let save_result = save_history(&items);
        assert!(save_result.is_ok());

        let load_result = load_history();
        assert!(load_result.is_ok());
        let loaded_history = load_result.unwrap();

        assert_eq!(loaded_history.len(), 3);
        assert_eq!(loaded_history[0].label, "Test App");
        assert_eq!(loaded_history[1].label, "Test Note");
        assert_eq!(loaded_history[2].label, "Test File");
        
        // Check that execution time was preserved
        assert!(loaded_history[0].ran_at.is_some());
        assert!(loaded_history[1].ran_at.is_none());
    }

    #[test]
    fn test_add_to_history_new_item() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        env::set_var("HOME", temp_path);

        let mut history = vec![];
        let item = create_test_item("New App", Handler::App, "/Applications/New.app");

        let result = add_to_history(&mut history, item.clone());
        assert!(result.is_ok());
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].label, "New App");
        assert!(history[0].ran_at.is_some());
    }

    #[test]
    fn test_add_to_history_duplicate_removal() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        env::set_var("HOME", temp_path);

        let mut history = vec![
            create_test_item("App 1", Handler::App, "/Applications/App1.app"),
            create_test_item("App 2", Handler::App, "/Applications/App2.app"),
            create_test_item("App 3", Handler::App, "/Applications/App3.app"),
        ];

        // Add duplicate of App 2
        let duplicate_item = create_test_item("App 2", Handler::App, "/Applications/App2.app");
        let result = add_to_history(&mut history, duplicate_item);
        
        assert!(result.is_ok());
        assert_eq!(history.len(), 3); // Should still be 3 items
        
        // App 2 should now be at the end (most recent)
        assert_eq!(history[2].label, "App 2");
        assert!(history[2].ran_at.is_some());
        
        // Other items should remain
        assert_eq!(history[0].label, "App 1");
        assert_eq!(history[1].label, "App 3");
    }

    #[test]
    fn test_add_to_history_different_handlers() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        env::set_var("HOME", temp_path);

        let mut history = vec![
            create_test_item("Test", Handler::App, "test"),
        ];

        // Add item with same label and value but different handler
        let note_item = create_test_item("Test", Handler::Note, "test");
        let result = add_to_history(&mut history, note_item);
        
        assert!(result.is_ok());
        assert_eq!(history.len(), 2); // Should be 2 items since handlers differ
        assert_eq!(history[0].handler, Handler::App);
        assert_eq!(history[1].handler, Handler::Note);
    }

    #[test]
    fn test_history_file_path_creation() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        env::set_var("HOME", temp_path);

        let path_result = history_file_path();
        assert!(path_result.is_ok());
        
        let path = path_result.unwrap();
        assert!(path.to_string_lossy().contains("grinta_history.json"));
    }

    #[test]
    fn test_load_corrupted_history() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        env::set_var("HOME", temp_path);

        // Create a corrupted history file
        let path = history_file_path().unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "invalid json content").unwrap();

        let result = load_history();
        assert!(result.is_ok());
        let history = result.unwrap();
        assert!(history.is_empty()); // Should return empty vec for corrupted data
    }

    #[test]
    fn test_history_preserves_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        env::set_var("HOME", temp_path);

        let mut item = create_test_item("Test File", Handler::File, "/path/to/test.txt");
        item.metadata.insert("size".to_string(), "1024".to_string());
        item.metadata.insert("type".to_string(), "text".to_string());

        let mut history = vec![];
        let result = add_to_history(&mut history, item);
        assert!(result.is_ok());

        let loaded_history = load_history().unwrap();
        assert_eq!(loaded_history.len(), 1);
        assert_eq!(loaded_history[0].metadata.get("size"), Some(&"1024".to_string()));
        assert_eq!(loaded_history[0].metadata.get("type"), Some(&"text".to_string()));
    }
}
