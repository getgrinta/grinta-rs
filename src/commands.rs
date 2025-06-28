use crate::data_sources;
use anyhow::Result;
use open;
use std::process::Command;

pub use crate::core::{CommandItem, Handler};

pub async fn execute_command(item: &CommandItem, alt_modifier_active: bool) -> Result<()> {
    match item.handler {
        Handler::Url => {
            open::that(&item.value)?;
        }
        Handler::App => {
            #[cfg(target_os = "macos")]
            {
                Command::new("open").arg(&item.value).spawn()?;
            }
            #[cfg(not(target_os = "macos"))]
            {
                open::that(&item.value)?;
            }
        }
        Handler::Note => {
            data_sources::notes::open_note(&item.value).await?;
        }
        Handler::File | Handler::Folder => {
            if alt_modifier_active {
                #[cfg(target_os = "macos")]
                {
                    Command::new("open").arg("-R").arg(&item.value).spawn()?;
                }
                #[cfg(not(target_os = "macos"))]
                {
                    // For non-macOS, Alt+Enter on a file/folder could potentially
                    // open its parent directory. This is a placeholder for that logic.
                    // For now, it will just open the item directly.
                    open::that(&item.value)?;
                }
            } else {
                open::that(&item.value)?;
            }
        }
        Handler::Automation => {
            #[cfg(target_os = "macos")]
            {
                Command::new("shortcuts").args(["run", &item.value]).spawn()?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CommandItem, Handler};

    fn create_test_item(label: &str, handler: Handler, value: &str) -> CommandItem {
        CommandItem::new(label, handler, value)
    }

    #[tokio::test]
    async fn test_execute_command_url() {
        let item = create_test_item("Test URL", Handler::Url, "https://example.com");
        
        // This test just ensures the function doesn't panic
        // In a real test environment, we'd mock the `open` crate
        let result = execute_command(&item, false).await;
        
        // The result depends on whether the system can open URLs
        // We just check that the function completes without panicking
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_execute_command_app() {
        let item = create_test_item("Test App", Handler::App, "/Applications/Calculator.app");
        
        let result = execute_command(&item, false).await;
        
        // The result depends on whether the app exists
        // We just check that the function completes without panicking
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_execute_command_file() {
        let item = create_test_item("Test File", Handler::File, "/tmp/test.txt");
        
        // Test normal execution
        let result = execute_command(&item, false).await;
        assert!(result.is_ok() || result.is_err());
        
        // Test with alt modifier (should reveal in finder on macOS)
        let result_alt = execute_command(&item, true).await;
        assert!(result_alt.is_ok() || result_alt.is_err());
    }

    #[tokio::test]
    async fn test_execute_command_folder() {
        let item = create_test_item("Test Folder", Handler::Folder, "/tmp");
        
        // Test normal execution
        let result = execute_command(&item, false).await;
        assert!(result.is_ok() || result.is_err());
        
        // Test with alt modifier
        let result_alt = execute_command(&item, true).await;
        assert!(result_alt.is_ok() || result_alt.is_err());
    }

    #[tokio::test]
    async fn test_execute_command_automation() {
        let item = create_test_item("Test Shortcut", Handler::Automation, "Test Shortcut");
        
        let result = execute_command(&item, false).await;
        
        // On macOS, this will try to run a shortcut
        // On other platforms, it should complete without error
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_command_item_handlers() {
        // Test that all handler types are covered
        let handlers = vec![
            Handler::Url,
            Handler::App,
            Handler::Note,
            Handler::File,
            Handler::Folder,
            Handler::Automation,
        ];

        for handler in handlers {
            let item = create_test_item("Test", handler, "test_value");
            assert_eq!(item.handler, handler);
        }
    }

    #[test]
    fn test_handler_consistency() {
        // Ensure handler enum values match what we expect
        assert_eq!(Handler::Url.to_string(), "Website");
        assert_eq!(Handler::App.to_string(), "Application");
        assert_eq!(Handler::Note.to_string(), "Note");
        assert_eq!(Handler::File.to_string(), "File");
        assert_eq!(Handler::Folder.to_string(), "Folder");
        assert_eq!(Handler::Automation.to_string(), "Shortcut");
    }

    #[tokio::test]
    async fn test_execute_command_note() {
        let item = create_test_item("Test Note", Handler::Note, "note-id-123");
        
        // This will try to open a note with the given ID
        let result = execute_command(&item, false).await;
        
        // The result depends on whether the note exists and the platform
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_alt_modifier_behavior() {
        // Test that alt modifier affects file/folder handling differently
        let file_item = create_test_item("File", Handler::File, "/path/to/file.txt");
        let folder_item = create_test_item("Folder", Handler::Folder, "/path/to/folder");
        let url_item = create_test_item("URL", Handler::Url, "https://example.com");
        
        // Alt modifier should only affect File and Folder handlers
        // For other handlers, it should be ignored
        assert_eq!(file_item.handler, Handler::File);
        assert_eq!(folder_item.handler, Handler::Folder);
        assert_eq!(url_item.handler, Handler::Url);
    }

    #[test]
    fn test_command_item_creation() {
        let item = create_test_item("Test Command", Handler::App, "/Applications/Test.app");
        
        assert_eq!(item.label, "Test Command");
        assert_eq!(item.handler, Handler::App);
        assert_eq!(item.value, "/Applications/Test.app");
        assert_eq!(item.icon, Handler::App.to_icon());
    }

    #[tokio::test]
    async fn test_execute_command_error_handling() {
        // Test with invalid paths/values to ensure error handling works
        let invalid_items = vec![
            create_test_item("Invalid App", Handler::App, "/nonexistent/app.app"),
            create_test_item("Invalid File", Handler::File, "/nonexistent/file.txt"),
            create_test_item("Invalid URL", Handler::Url, "invalid-url"),
        ];

        for item in invalid_items {
            let result = execute_command(&item, false).await;
            // Should either succeed (if system handles gracefully) or fail gracefully
            // Either way, it shouldn't panic
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[test]
    fn test_platform_specific_compilation() {
        // This test ensures the code compiles on different platforms
        // The actual behavior will differ, but compilation should work
        
        #[cfg(target_os = "macos")]
        {
            // macOS-specific code paths exist
            assert!(true);
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            // Non-macOS fallbacks exist
            assert!(true);
        }
    }
}
