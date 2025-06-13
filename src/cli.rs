use crate::core::CommandItem;
use crate::data_sources;

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::Serialize;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub search_command: Option<SearchCommand>,
}

#[derive(Subcommand)]
pub enum SearchCommand {
    /// Search for commands
    Search {
        /// Query string to search for
        query: String,
    },
}

#[derive(Serialize)]
struct CommandOutput {
    label: String,
    handler: String,
    value: String,
    icon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    base64_icon: Option<String>,
}

impl From<&CommandItem> for CommandOutput {
    fn from(item: &CommandItem) -> Self {
        Self {
            label: item.label.clone(),
            handler: item.handler.to_string().into(),
            value: item.value.clone(),
            icon: item.icon.clone(),
            base64_icon: item.base64_icon.clone(),
        }
    }
}

pub async fn run_search_command(command: SearchCommand) -> Result<()> {
    let SearchCommand::Search { query } = command;
    let local_items_handle = tokio::spawn(data_sources::get_all_items(true));
    let suggestions_handle =
        tokio::spawn(data_sources::web_search::get_web_search_suggestions(query.clone()));

    let local_items = local_items_handle.await.unwrap_or_default();
    let suggestions = match suggestions_handle.await {
        Ok(Ok(s)) => s,
        _ => vec![],
    };

    let direct_search = data_sources::web_search::create_web_search_command(&query);

    let filtered_items: Vec<CommandItem> = local_items
        .into_iter()
        .filter(|item| {
            let lower_query = query.to_lowercase();
            item.label.to_lowercase().contains(&lower_query)
                || item.value.to_lowercase().contains(&lower_query)
        })
        .collect();

    let mut all_items = vec![direct_search];
    all_items.extend(suggestions);
    all_items.extend(filtered_items);

    let output: Vec<CommandOutput> = all_items.iter().map(CommandOutput::from).collect();
    let json = serde_json::to_string_pretty(&output)?;
    println!("{}", json);
    Ok(())
}
