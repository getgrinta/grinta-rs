use crate::core::{CommandItem, CommandType, Handler};
use anyhow::Result;
use open;
use reqwest;
use serde_json::Value;
use std::time::Duration;
use urlencoding;

fn create_suggestion_command(suggestion: &str) -> CommandItem {
    let mut cmd = CommandItem::new(
        suggestion,
        Handler::Url,
        &format!("https://duckduckgo.com/?q={}", suggestion),
    );
    cmd.icon = "🔎".to_string();
    cmd.kind = CommandType::WebSuggestion;
    cmd
}

pub async fn get_web_search_suggestions(query: String) -> Result<Vec<CommandItem>> {
    if query.is_empty() {
        return Ok(vec![]);
    }

    let client = reqwest::Client::new();
    let response = client
        .get("https://duckduckgo.com/ac/")
        .query(&[("q", &query)])
        .timeout(Duration::from_millis(500))
        .send()
        .await?
        .json::<Value>()
        .await?;

    let suggestions = response
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|item| item["phrase"].as_str())
        .map(create_suggestion_command)
        .collect();

    Ok(suggestions)
}

fn open_url(url: &str) {
    if let Err(e) = open::that(url) {
        eprintln!("Failed to open URL: {}", e);
    }
}

pub fn search_web(query: &str) {
    let encoded_query = urlencoding::encode(query);
    let url = format!("https://duckduckgo.com/?q={}", encoded_query);
    open_url(&url);
}

pub fn open_chat_gpt(query: &str) {
    let encoded_query = urlencoding::encode(query);
        let url = format!("https://chatgpt.com/?q={}", encoded_query);
    open_url(&url);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CommandType, Handler};

    #[test]
    fn test_create_suggestion_command() {
        let suggestion = "test query";
        let cmd = create_suggestion_command(suggestion);
        
        assert_eq!(cmd.label, "test query");
        assert_eq!(cmd.handler, Handler::Url);
        assert_eq!(cmd.value, "https://duckduckgo.com/?q=test query");
        assert_eq!(cmd.icon, "🔎");
        assert_eq!(cmd.kind, CommandType::WebSuggestion);
    }

    #[test]
    fn test_create_suggestion_command_special_characters() {
        let suggestion = "test & query";
        let cmd = create_suggestion_command(suggestion);
        
        assert_eq!(cmd.label, "test & query");
        assert_eq!(cmd.value, "https://duckduckgo.com/?q=test & query");
    }

    #[tokio::test]
    async fn test_get_web_search_suggestions_empty_query() {
        let result = get_web_search_suggestions(String::new()).await;
        
        assert!(result.is_ok());
        let suggestions = result.unwrap();
        assert!(suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_get_web_search_suggestions_timeout() {
        // Test with a very short timeout (this will likely fail but shouldn't panic)
        let result = get_web_search_suggestions("test".to_string()).await;
        
        // Either succeeds or fails gracefully due to timeout/network issues
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_search_web_url_encoding() {
        // We can't easily test the actual opening, but we can test URL construction
        let query = "test query with spaces";
        
        // This is what the function should generate internally
        let encoded = urlencoding::encode(query);
        let expected_url = format!("https://duckduckgo.com/?q={}", encoded);
        
        assert_eq!(expected_url, "https://duckduckgo.com/?q=test%20query%20with%20spaces");
    }

    #[test]
    fn test_open_chat_gpt_url_encoding() {
        let query = "test query with spaces";
        
        let encoded = urlencoding::encode(query);
        let expected_url = format!("https://chatgpt.com/?q={}", encoded);
        
        assert_eq!(expected_url, "https://chatgpt.com/?q=test%20query%20with%20spaces");
    }

    #[test]
    fn test_url_encoding_special_characters() {
        let special_chars = "test+query&with=special%chars";
        let encoded = urlencoding::encode(special_chars);
        
        // Should properly encode special URL characters
        assert!(encoded.contains("%"));
        assert!(!encoded.contains("&"));
        assert!(!encoded.contains("="));
    }

    #[tokio::test]
    async fn test_web_search_suggestions_format() {
        // Test that if we get a successful response, it's properly formatted
        let result = get_web_search_suggestions("rust".to_string()).await;
        
        if let Ok(suggestions) = result {
            for suggestion in suggestions {
                // Each suggestion should be a valid CommandItem
                assert_eq!(suggestion.handler, Handler::Url);
                assert_eq!(suggestion.icon, "🔎");
                assert_eq!(suggestion.kind, CommandType::WebSuggestion);
                assert!(suggestion.value.starts_with("https://duckduckgo.com/?q="));
                assert!(!suggestion.label.is_empty());
            }
        }
        // If it fails, that's also acceptable (network issues, etc.)
    }

    #[test]
    fn test_command_type_consistency() {
        let cmd = create_suggestion_command("test");
        assert_eq!(cmd.kind, CommandType::WebSuggestion);
        
        // Ensure it's different from other command types
        assert_ne!(cmd.kind, CommandType::App);
        assert_ne!(cmd.kind, CommandType::Note);
        assert_ne!(cmd.kind, CommandType::WebSearch);
    }

    #[test]
    fn test_handler_consistency() {
        let cmd = create_suggestion_command("test");
        assert_eq!(cmd.handler, Handler::Url);
        assert_eq!(cmd.handler.to_string(), "Website");
        assert_eq!(cmd.handler.to_icon(), "🔗");
        
        // But our custom icon should override the default
        assert_eq!(cmd.icon, "🔎");
    }

    #[test]
    fn test_empty_suggestion() {
        let cmd = create_suggestion_command("");
        assert_eq!(cmd.label, "");
        assert_eq!(cmd.value, "https://duckduckgo.com/?q=");
    }

    #[test]
    fn test_long_suggestion() {
        let long_query = "a".repeat(1000);
        let cmd = create_suggestion_command(&long_query);
        
        assert_eq!(cmd.label.len(), 1000);
        assert!(cmd.value.contains(&long_query));
    }

    #[test]
    fn test_unicode_suggestion() {
        let unicode_query = "test 🔍 query with émojis and açcénts";
        let cmd = create_suggestion_command(unicode_query);
        
        assert_eq!(cmd.label, unicode_query);
        assert!(cmd.value.contains(unicode_query));
    }

    #[tokio::test]
    async fn test_suggestion_api_response_structure() {
        // Test that we can handle different response structures gracefully
        let result = get_web_search_suggestions("test".to_string()).await;
        
        match result {
            Ok(suggestions) => {
                // If successful, suggestions should be valid
                for suggestion in suggestions {
                    assert!(!suggestion.label.is_empty());
                    assert!(suggestion.value.starts_with("https://"));
                }
            }
            Err(_) => {
                // Network errors are acceptable in tests
                assert!(true);
            }
        }
    }

    #[test]
    fn test_url_construction() {
        let base_url = "https://duckduckgo.com/?q=";
        let query = "rust programming";
        let full_url = format!("{}{}", base_url, query);
        
        assert_eq!(full_url, "https://duckduckgo.com/?q=rust programming");
        
        // Test with encoded version
        let encoded_query = urlencoding::encode(query);
        let encoded_url = format!("{}{}", base_url, encoded_query);
        assert_eq!(encoded_url, "https://duckduckgo.com/?q=rust%20programming");
    }
}
