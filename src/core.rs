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

#[derive(Clone, Serialize, Deserialize)]
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
