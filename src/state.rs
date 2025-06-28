use crate::core::CommandItem;
use ratatui::widgets::TableState;
use tui_textarea::TextArea;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub struct AppState<'a> {
    pub query: TextArea<'a>,
    pub items: Vec<CommandItem>,
    pub filtered_items: Vec<CommandItem>,
    pub table_state: TableState,
    pub history: Vec<CommandItem>,
    pub fs_items: Vec<CommandItem>,
    pub web_items: Vec<CommandItem>,
    pub error_message: Option<String>,
}

impl<'a> AppState<'a> {
    pub fn new(history: Vec<CommandItem>, items: Vec<CommandItem>) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let mut state = Self {
            query: TextArea::default(),
            items,
            filtered_items: vec![],
            table_state,
            history,
            fs_items: vec![],
            web_items: vec![],
            error_message: None,
        };
        state.filter_items();
        state
    }

    pub fn filter_items(&mut self) {
        let query = self.query.lines().join(" ").trim().to_string();
        if query.is_empty() {
            self.filtered_items = self.history.clone();
            self.filtered_items.reverse();
        } else {
            let matcher = SkimMatcherV2::default();
            
            // Filter static items using fuzzy matching
            let mut static_filtered: Vec<CommandItem> = self.items
                .iter()
                .filter(|item| {
                    item.label.to_lowercase().contains(&query.to_lowercase())
                        || item.value.to_lowercase().contains(&query.to_lowercase())
                        || matcher.fuzzy_match(&item.label, &query).unwrap_or(0) > 0
                        || matcher.fuzzy_match(&item.value, &query).unwrap_or(0) > 0
                })
                .cloned()
                .collect();

            // Filter dynamic items (FS + Web)
            let mut fs_filtered: Vec<CommandItem> = self.fs_items
                .iter()
                .filter(|item| {
                    item.label.to_lowercase().contains(&query.to_lowercase())
                        || item.value.to_lowercase().contains(&query.to_lowercase())
                        || matcher.fuzzy_match(&item.label, &query).unwrap_or(0) > 0
                        || matcher.fuzzy_match(&item.value, &query).unwrap_or(0) > 0
                })
                .cloned()
                .collect();

            let mut web_filtered: Vec<CommandItem> = self.web_items
                .iter()
                .filter(|item| {
                    item.label.to_lowercase().contains(&query.to_lowercase())
                        || item.value.to_lowercase().contains(&query.to_lowercase())
                        || matcher.fuzzy_match(&item.label, &query).unwrap_or(0) > 0
                        || matcher.fuzzy_match(&item.value, &query).unwrap_or(0) > 0
                })
                .cloned()
                .collect();
            
            // Combine all dynamic results: FS + Web suggestions
            let mut new_filtered = Vec::new();
            new_filtered.append(&mut static_filtered);
            new_filtered.append(&mut fs_filtered);
            new_filtered.append(&mut web_filtered);
            
            self.filtered_items = new_filtered;
            
            // Sort by fuzzy match score FIRST, then by type as tie-breaker
            self.filtered_items.sort_by(|a, b| {
                use crate::core::CommandType;
                
                // Primary sort: by fuzzy match score (higher score = better match)
                let a_label_fuzzy = matcher.fuzzy_match(&a.label, &query).unwrap_or(0);
                let a_value_fuzzy = matcher.fuzzy_match(&a.value, &query).unwrap_or(0);
                let a_fuzzy = a_label_fuzzy.max(a_value_fuzzy);
                
                let b_label_fuzzy = matcher.fuzzy_match(&b.label, &query).unwrap_or(0);
                let b_value_fuzzy = matcher.fuzzy_match(&b.value, &query).unwrap_or(0);
                let b_fuzzy = b_label_fuzzy.max(b_value_fuzzy);
                
                match b_fuzzy.cmp(&a_fuzzy) {
                    std::cmp::Ordering::Equal => {
                        // Tie-breaker: prefer local items over web suggestions
                        let a_priority = match a.kind {
                            CommandType::App => 1,
                            CommandType::Note => 1,
                            CommandType::Bookmark => 1,
                            CommandType::Unknown => 1,
                            CommandType::WebSearch => 2,
                            CommandType::WebSuggestion => 2,
                        };
                        
                        let b_priority = match b.kind {
                            CommandType::App => 1,
                            CommandType::Note => 1,
                            CommandType::Bookmark => 1,
                            CommandType::Unknown => 1,
                            CommandType::WebSearch => 2,
                            CommandType::WebSuggestion => 2,
                        };
                        
                        match a_priority.cmp(&b_priority) {
                            std::cmp::Ordering::Equal => a.label.cmp(&b.label),
                            other => other
                        }
                    }
                    other => other
                }
            });
        }

        if self.filtered_items.is_empty() {
            self.table_state.select(None);
        } else {
            if self.table_state.selected().is_none() {
                self.table_state.select(Some(0));
            }
        }
    }

    pub fn get_selected_item(&self) -> Option<&CommandItem> {
        self.table_state
            .selected()
            .and_then(|i| self.filtered_items.get(i))
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CommandItem, Handler, CommandType};

    fn create_test_item(label: &str, handler: Handler, value: &str) -> CommandItem {
        let mut item = CommandItem::new(label, handler, value);
        item.kind = match handler {
            Handler::App => CommandType::App,
            Handler::Note => CommandType::Note,
            Handler::Url => CommandType::WebSearch,
            _ => CommandType::Unknown,
        };
        item
    }

    fn create_web_item(label: &str) -> CommandItem {
        let mut item = CommandItem::new(label, Handler::Url, &format!("https://example.com/{}", label));
        item.kind = CommandType::WebSuggestion;
        item
    }

    #[test]
    fn test_app_state_new() {
        let history = vec![create_test_item("Test App", Handler::App, "test")];
        let items = vec![create_test_item("Another App", Handler::App, "another")];
        
        let state = AppState::new(history.clone(), items.clone());
        
        assert_eq!(state.history, history);
        assert_eq!(state.items, items);
        assert!(state.fs_items.is_empty());
        assert!(state.web_items.is_empty());
        // filtered_items will contain reversed history due to filter_items() being called in new()
        assert_eq!(state.filtered_items.len(), 1);
        assert_eq!(state.filtered_items[0].label, "Test App");
        assert!(state.query.is_empty());
        assert!(state.error_message.is_none());
        assert_eq!(state.table_state.selected(), Some(0)); // Auto-selected first item
    }

    #[test]
    fn test_filter_items_empty_query() {
        let history = vec![
            create_test_item("Recent App", Handler::App, "recent"),
            create_test_item("Old App", Handler::App, "old"),
        ];
        let mut state = AppState::new(history, vec![]);
        
        state.filter_items();
        
        // Should show history when query is empty (reversed order)
        assert_eq!(state.filtered_items.len(), 2);
        assert_eq!(state.filtered_items[0].label, "Old App");
        assert_eq!(state.filtered_items[1].label, "Recent App");
    }

    #[test]
    fn test_filter_items_with_query() {
        let items = vec![
            create_test_item("Cursor", Handler::App, "cursor"),
            create_test_item("Chrome", Handler::App, "chrome"),
            create_test_item("Calculator", Handler::App, "calculator"),
        ];
        let mut state = AppState::new(vec![], items);
        
        // Set query
        state.query.insert_str("cur");
        state.filter_items();
        
        // Should filter and sort by fuzzy match score
        assert!(!state.filtered_items.is_empty());
        // "Cursor" should rank higher than "Calculator" for query "cur"
        let cursor_pos = state.filtered_items.iter().position(|item| item.label == "Cursor");
        let calc_pos = state.filtered_items.iter().position(|item| item.label == "Calculator");
        
        if let (Some(cursor), Some(calc)) = (cursor_pos, calc_pos) {
            assert!(cursor < calc, "Cursor should rank higher than Calculator for query 'cur'");
        }
    }

    #[test]
    fn test_fuzzy_matching_priority() {
        let items = vec![
            create_test_item("notes", Handler::Url, "notes"),
            create_test_item("Postgres", Handler::App, "postgres"),
            create_test_item("Note Taking", Handler::Note, "note-taking"),
        ];
        let mut state = AppState::new(vec![], items);
        
        state.query.insert_str("notes");
        state.filter_items();
        
        // "notes" should be first due to exact match
        assert!(!state.filtered_items.is_empty());
        assert_eq!(state.filtered_items[0].label, "notes");
    }

    #[test]
    fn test_filter_combines_all_sources() {
        let items = vec![create_test_item("App Test", Handler::App, "app")];
        let fs_items = vec![create_test_item("file_test.txt", Handler::File, "/path/file_test.txt")];
        let web_items = vec![create_web_item("web test")];
        
        let mut state = AppState::new(vec![], items);
        state.fs_items = fs_items;
        state.web_items = web_items;
        
        state.query.insert_str("test");
        state.filter_items();
        
        // Should include items from all sources
        assert_eq!(state.filtered_items.len(), 3);
        
        let labels: Vec<&str> = state.filtered_items.iter().map(|item| item.label.as_str()).collect();
        assert!(labels.contains(&"App Test"));
        assert!(labels.contains(&"file_test.txt"));
        assert!(labels.contains(&"web test"));
    }

    #[test]
    fn test_local_vs_web_priority() {
        let items = vec![create_test_item("test app", Handler::App, "test")];
        let web_items = vec![create_web_item("test")];
        
        let mut state = AppState::new(vec![], items);
        state.web_items = web_items;
        
        state.query.insert_str("test");
        state.filter_items();
        
        // Local items should come before web suggestions for same fuzzy score
        assert_eq!(state.filtered_items.len(), 2);
        assert_eq!(state.filtered_items[0].label, "test app");
        assert_eq!(state.filtered_items[1].label, "test");
    }

    #[test]
    fn test_get_selected_item() {
        // Test with history items (which are shown when query is empty)
        let history = vec![
            create_test_item("First", Handler::App, "first"),
            create_test_item("Second", Handler::App, "second"),
        ];
        let mut state = AppState::new(history, vec![]);
        
        // With empty query, should show history and auto-select first item
        assert_eq!(state.filtered_items.len(), 2);
        let selected = state.get_selected_item();
        assert!(selected.is_some());
        
        // Select second item
        state.table_state.select(Some(1));
        let selected = state.get_selected_item();
        assert!(selected.is_some());
        
        // Invalid selection
        state.table_state.select(Some(10));
        assert!(state.get_selected_item().is_none());
        
        // Test with empty filtered items
        let mut empty_state = AppState::new(vec![], vec![]);
        empty_state.query.insert_str("nonexistent");
        empty_state.filter_items();
        
        assert!(empty_state.filtered_items.is_empty());
        assert!(empty_state.get_selected_item().is_none());
    }

    #[test]
    fn test_error_handling() {
        let mut state = AppState::new(vec![], vec![]);
        
        // Initially no error
        assert!(state.error_message.is_none());
        
        // Set error
        state.set_error("Test error".to_string());
        assert_eq!(state.error_message, Some("Test error".to_string()));
        
        // Clear error
        state.clear_error();
        assert!(state.error_message.is_none());
    }

    #[test]
    fn test_filter_empty_items() {
        let mut state = AppState::new(vec![], vec![]);
        
        state.query.insert_str("anything");
        state.filter_items();
        
        assert!(state.filtered_items.is_empty());
    }

    #[test]
    fn test_case_insensitive_matching() {
        let items = vec![
            create_test_item("CURSOR", Handler::App, "cursor"),
            create_test_item("cursor", Handler::App, "cursor2"),
            create_test_item("Cursor", Handler::App, "cursor3"),
        ];
        let mut state = AppState::new(vec![], items);
        
        state.query.insert_str("cursor");
        state.filter_items();
        
        // All variants should match
        assert_eq!(state.filtered_items.len(), 3);
    }

    #[test]
    fn test_partial_matching() {
        let items = vec![
            create_test_item("Visual Studio Code", Handler::App, "vscode"),
            create_test_item("IntelliJ IDEA", Handler::App, "idea"),
            create_test_item("Sublime Text", Handler::App, "sublime"),
        ];
        let mut state = AppState::new(vec![], items);
        
        state.query.insert_str("code");
        state.filter_items();
        
        // Should match "Visual Studio Code"
        assert!(!state.filtered_items.is_empty());
        assert!(state.filtered_items.iter().any(|item| item.label.contains("Code")));
    }

    #[test]
    fn test_sort_stability() {
        // Test that items with same fuzzy score are sorted consistently
        let items = vec![
            create_test_item("App A", Handler::App, "a"),
            create_test_item("App B", Handler::App, "b"),
            create_test_item("App C", Handler::App, "c"),
        ];
        let mut state = AppState::new(vec![], items);
        
        // Query that doesn't match any item well (low scores)
        state.query.insert_str("xyz");
        state.filter_items();
        
        // Should maintain some consistent order even with low scores
        let first_run = state.filtered_items.clone();
        
        state.filter_items(); // Run again
        assert_eq!(state.filtered_items, first_run);
    }

    #[test]
    fn test_mixed_handler_types() {
        let items = vec![
            create_test_item("test.txt", Handler::File, "/path/test.txt"),
            create_test_item("Test App", Handler::App, "test_app"),
            create_test_item("Test Note", Handler::Note, "test_note"),
            create_test_item("Test Folder", Handler::Folder, "/path/test_folder"),
        ];
        let mut state = AppState::new(vec![], items);
        
        state.query.insert_str("test");
        state.filter_items();
        
        // All should match
        assert_eq!(state.filtered_items.len(), 4);
        
        // Verify all handler types are present
        let handlers: Vec<Handler> = state.filtered_items.iter().map(|item| item.handler).collect();
        assert!(handlers.contains(&Handler::File));
        assert!(handlers.contains(&Handler::App));
        assert!(handlers.contains(&Handler::Note));
        assert!(handlers.contains(&Handler::Folder));
    }
}
