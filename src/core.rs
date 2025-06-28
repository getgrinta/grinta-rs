use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Handler {
    App,
    Note,
    Url,
    Automation,
    Folder,
    File,
}

impl Handler {
    pub fn to_string(&self) -> &'static str {
        match self {
            Handler::Url => "Website",
            Handler::App => "Application",
            Handler::Note => "Note",
            Handler::File => "File",
            Handler::Folder => "Folder",
            Handler::Automation => "Shortcut",
        }
    }

    pub fn to_icon(&self) -> &'static str {
        match self {
            Handler::Url => "🔗",
            Handler::App => "📱",
            Handler::Note => "📝",
            Handler::File => "📄",
            Handler::Folder => "📁",
            Handler::Automation => "⚡",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandType {
    App,
    Bookmark,
    Note,
    WebSearch,
    WebSuggestion,
    Unknown,
}

impl Default for CommandType {
    fn default() -> Self {
        CommandType::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandItem {
    pub label: String,
    pub handler: Handler,
    pub value: String,
    pub icon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ran_at: Option<DateTime<Local>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64_icon: Option<String>,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub kind: CommandType,
}

impl CommandItem {
    pub fn new(label: &str, handler: Handler, value: &str) -> Self {
        Self {
            label: label.to_string(),
            handler,
            value: value.to_string(),
            icon: handler.to_icon().to_string(),
            ran_at: None,
            base64_icon: None,
            metadata: std::collections::HashMap::new(),
            kind: CommandType::Unknown,
        }
    }

    /// Mark this command as executed with the current timestamp
    pub fn mark_executed(&mut self) {
        self.ran_at = Some(Local::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    #[test]
    fn test_handler_to_string() {
        assert_eq!(Handler::App.to_string(), "Application");
        assert_eq!(Handler::Note.to_string(), "Note");
        assert_eq!(Handler::Url.to_string(), "Website");
        assert_eq!(Handler::File.to_string(), "File");
        assert_eq!(Handler::Folder.to_string(), "Folder");
        assert_eq!(Handler::Automation.to_string(), "Shortcut");
    }

    #[test]
    fn test_handler_to_icon() {
        assert_eq!(Handler::App.to_icon(), "📱");
        assert_eq!(Handler::Note.to_icon(), "📝");
        assert_eq!(Handler::Url.to_icon(), "🔗");
        assert_eq!(Handler::File.to_icon(), "📄");
        assert_eq!(Handler::Folder.to_icon(), "📁");
        assert_eq!(Handler::Automation.to_icon(), "⚡");
    }

    #[test]
    fn test_handler_ordering() {
        let mut handlers = vec![Handler::Url, Handler::App, Handler::Note, Handler::File, Handler::Folder, Handler::Automation];
        handlers.sort();
        assert_eq!(handlers, vec![Handler::App, Handler::Note, Handler::Url, Handler::Automation, Handler::Folder, Handler::File]);
    }

    #[test]
    fn test_command_type_default() {
        assert_eq!(CommandType::default(), CommandType::Unknown);
    }

    #[test]
    fn test_command_item_new() {
        let item = CommandItem::new("Test App", Handler::App, "/Applications/Test.app");
        
        assert_eq!(item.label, "Test App");
        assert_eq!(item.handler, Handler::App);
        assert_eq!(item.value, "/Applications/Test.app");
        assert_eq!(item.icon, "📱");
        assert!(item.ran_at.is_none());
        assert!(item.base64_icon.is_none());
        assert!(item.metadata.is_empty());
        assert_eq!(item.kind, CommandType::Unknown);
    }

    #[test]
    fn test_command_item_mark_executed() {
        let mut item = CommandItem::new("Test", Handler::App, "test");
        assert!(item.ran_at.is_none());
        
        let before = Local::now();
        item.mark_executed();
        let after = Local::now();
        
        assert!(item.ran_at.is_some());
        let ran_at = item.ran_at.unwrap();
        assert!(ran_at >= before && ran_at <= after);
    }

    #[test]
    fn test_command_item_serialization() {
        let mut item = CommandItem::new("Test Note", Handler::Note, "note-id-123");
        item.mark_executed();
        item.base64_icon = Some("base64data".to_string());
        item.metadata.insert("folder".to_string(), "Work".to_string());
        item.kind = CommandType::Note;

        let json = serde_json::to_string(&item).unwrap();
        let deserialized: CommandItem = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.label, item.label);
        assert_eq!(deserialized.handler, item.handler);
        assert_eq!(deserialized.value, item.value);
        assert_eq!(deserialized.base64_icon, item.base64_icon);
        assert_eq!(deserialized.metadata, item.metadata);
        assert_eq!(deserialized.kind, item.kind);
    }

    #[test]
    fn test_command_item_clone() {
        let mut original = CommandItem::new("Original", Handler::File, "/path/to/file");
        original.mark_executed();
        original.metadata.insert("type".to_string(), "document".to_string());

        let cloned = original.clone();
        
        assert_eq!(cloned.label, original.label);
        assert_eq!(cloned.handler, original.handler);
        assert_eq!(cloned.value, original.value);
        assert_eq!(cloned.ran_at, original.ran_at);
        assert_eq!(cloned.metadata, original.metadata);
    }

    #[test]
    fn test_command_item_equality() {
        let item1 = CommandItem::new("Test", Handler::App, "test");
        let item2 = CommandItem::new("Test", Handler::App, "test");
        let item3 = CommandItem::new("Different", Handler::App, "test");

        assert_eq!(item1, item2);
        assert_ne!(item1, item3);
    }

    #[test]
    fn test_command_item_with_metadata() {
        let mut item = CommandItem::new("Document", Handler::File, "/path/doc.pdf");
        item.metadata.insert("size".to_string(), "1024".to_string());
        item.metadata.insert("type".to_string(), "pdf".to_string());

        assert_eq!(item.metadata.get("size"), Some(&"1024".to_string()));
        assert_eq!(item.metadata.get("type"), Some(&"pdf".to_string()));
        assert_eq!(item.metadata.len(), 2);
    }
}
