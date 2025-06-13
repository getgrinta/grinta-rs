use crate::core::{CommandItem, CommandType, Handler};
use anyhow::Result;
use open;
use reqwest;
use serde_json::Value;
use std::time::Duration;
use urlencoding;

pub fn create_web_search_command(query: &str) -> CommandItem {
    let mut cmd = CommandItem::new(
        &format!("Search for \"{}\"", query),
        Handler::Url,
        &format!("https://duckduckgo.com/?q={}", query),
    );
    cmd.icon = "🌐".to_string();
    cmd
}

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
