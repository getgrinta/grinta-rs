use crate::core::CommandItem;
use crate::data_sources;

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_json::json;
use std::io::{self, Write};
use tokio::sync::mpsc;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

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
struct StreamResponse {
    #[serde(rename = "type")]
    response_type: String,
    data: CommandOutput,
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

fn stream_result(item: &CommandItem, result_type: &str) -> Result<()> {
    let response = StreamResponse {
        response_type: result_type.to_string(),
        data: CommandOutput::from(item),
    };
    
    let json = serde_json::to_string(&response)?;
    println!("{}", json);
    io::stdout().flush()?;
    Ok(())
}

pub async fn run_search_command(command: SearchCommand) -> Result<()> {
    let result = run_search_command_inner(command).await;
    
    // Always send completion marker
    let completion = match &result {
        Ok(_) => json!({
            "type": "completion",
            "status": "success"
        }),
        Err(e) => json!({
            "type": "completion",
            "status": "error",
            "error": e.to_string()
        }),
    };
    
    println!("{}", serde_json::to_string(&completion)?);
    io::stdout().flush()?;
    
    result
}

async fn run_search_command_inner(command: SearchCommand) -> Result<()> {
    let SearchCommand::Search { query } = command;
    
    // Create channel for collecting results
    let (tx, mut rx) = mpsc::channel::<(CommandItem, String)>(100);
    
    let lower_query = query.to_lowercase();
    
    // Spawn separate tasks for each data source
    let handles = vec![
        // macOS Applications
        {
            let tx = tx.clone();
            let query = lower_query.clone();
            tokio::spawn(async move {
                #[cfg(target_os = "macos")]
                {
                    let applications_dirs = vec!["/Applications", "/System/Applications", "/System/Applications/Utilities"];
                    for dir in applications_dirs {
                        if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
                            while let Ok(Some(entry)) = entries.next_entry().await {
                                let path = entry.path();
                                if path.extension().and_then(|s| s.to_str()) == Some("app") {
                                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                                        if name.to_lowercase().contains(&query) {
                                            let path_str = path.to_str().unwrap_or("");
                                            let mut item = crate::core::CommandItem::new(name, crate::core::Handler::App, path_str);
                                            // Extract icon for CLI results
                                            item.base64_icon = crate::icons::extract_app_icon(path_str).await;
                                            let _ = tx.send((item, "app".to_string())).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            })
        },
        
        // Notes
        {
            let tx = tx.clone();
            let query = lower_query.clone();
            tokio::spawn(async move {
                #[cfg(target_os = "macos")]
                {
                    let notes = data_sources::notes::get_notes().await;
                    for note in notes {
                        if note.label.to_lowercase().contains(&query) 
                            || note.value.to_lowercase().contains(&query) 
                        {
                            let _ = tx.send((note, "note".to_string())).await;
                        }
                    }
                }
            })
        },
        
        // Bookmarks
        {
            let tx = tx.clone();
            let query = lower_query.clone();
            tokio::spawn(async move {
                let bookmarks = data_sources::bookmarks::get_browser_bookmarks().await;
                for bookmark in bookmarks {
                    if bookmark.label.to_lowercase().contains(&query) 
                        || bookmark.value.to_lowercase().contains(&query) 
                    {
                        let _ = tx.send((bookmark, "bookmark".to_string())).await;
                    }
                }
            })
        },
        
        // Automation/Shortcuts
        {
            let tx = tx.clone();
            let query = lower_query.clone();
            tokio::spawn(async move {
                #[cfg(target_os = "macos")]
                {
                    let shortcuts = data_sources::automation::get_shortcuts().await;
                    for shortcut in shortcuts {
                        if shortcut.label.to_lowercase().contains(&query) 
                            || shortcut.value.to_lowercase().contains(&query) 
                        {
                            let _ = tx.send((shortcut, "shortcut".to_string())).await;
                        }
                    }
                }
            })
        },
        
        // File System Search  
        {
            let tx = tx.clone();
            let query_fs = query.clone();
            tokio::spawn(async move {
                let fs_items = data_sources::fs::fast_file_search(&query_fs, 5).await;
                for item in fs_items {
                    let _ = tx.send((item, "file".to_string())).await;
                }
            })
        },

        // Web suggestions
        {
            let tx = tx.clone();
            let query_web = query.clone();
            tokio::spawn(async move {
                if let Ok(suggestions) = data_sources::web_search::get_web_search_suggestions(query_web).await {
                    for suggestion in suggestions {
                        let _ = tx.send((suggestion, "web_suggestion".to_string())).await;
                    }
                }
            })
        },
    ];
    
    // Drop the original sender so the receiver knows when all tasks are done
    drop(tx);
    
    // Collect all results first
    let mut all_results = Vec::new();
    while let Some((item, result_type)) = rx.recv().await {
        all_results.push((item, result_type));
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        let _ = handle.await;
    }
    
    // Sort results using fuzzy matching
    let matcher = SkimMatcherV2::default();
    let mut scored_results: Vec<((CommandItem, String), i64)> = all_results
        .into_iter()
        .filter_map(|(item, result_type)| {
            // Try fuzzy matching on both label and value
            let label_score = matcher.fuzzy_match(&item.label, &query).unwrap_or(0);
            let value_score = matcher.fuzzy_match(&item.value, &query).unwrap_or(0);
            let max_score = label_score.max(value_score);
            
            if max_score > 0 {
                Some(((item, result_type), max_score))
            } else {
                None
            }
        })
        .collect();
    
    // Sort by combined score: fuzzy match score + type priority bonus
    scored_results.sort_by(|a, b| {
        use crate::core::CommandType;
        
        // Calculate type priority bonus (higher bonus for preferred types)
        // Increased bonuses to make type priority more significant
        let a_type_bonus = match a.0.0.kind {
            CommandType::App => 200,       // Apps get very high bonus
            CommandType::Note => 150,      // Notes get high bonus  
            CommandType::Bookmark => 100,  // Bookmarks get medium bonus
            CommandType::Unknown => 50,    // Files get small bonus
            CommandType::WebSearch => 25,  // Web search gets tiny bonus
            CommandType::WebSuggestion => 0, // Web suggestions get no bonus
        };
        
        let b_type_bonus = match b.0.0.kind {
            CommandType::App => 200,
            CommandType::Note => 150,
            CommandType::Bookmark => 100,
            CommandType::Unknown => 50,
            CommandType::WebSearch => 25,
            CommandType::WebSuggestion => 0,
        };
        
        // Combined score = fuzzy score + type bonus
        let a_combined_score = a.1 + a_type_bonus;
        let b_combined_score = b.1 + b_type_bonus;
        
        // Sort by combined score (descending), then alphabetically for stable sorting
        match b_combined_score.cmp(&a_combined_score) {
            std::cmp::Ordering::Equal => a.0.0.label.to_lowercase().cmp(&b.0.0.label.to_lowercase()),
            other => other
        }
    });
    
    // Stream sorted results
    for ((item, result_type), _score) in scored_results {
        stream_result(&item, &result_type)?;
    }
    
    Ok(())
}
