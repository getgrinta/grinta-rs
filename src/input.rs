use crate::{
    core::{CommandItem, Handler},
    data_sources,
    history,
    state::AppState,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;
use std::sync::atomic::{AtomicU64, Ordering};

// Global counter to track search generations and cancel old searches
static SEARCH_GENERATION: AtomicU64 = AtomicU64::new(0);
static WEB_SEARCH_GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn handle_key_event(
    key: KeyEvent,
    app_state: &mut AppState,
    fs_tx: mpsc::Sender<Vec<CommandItem>>,
    web_tx: mpsc::Sender<Vec<CommandItem>>,
    refresh_tx: mpsc::Sender<()>,
    error_tx: Option<mpsc::Sender<String>>,
) -> bool {
    match key.code {
        KeyCode::Esc => return true, // Signal to exit
        KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => return true,
        KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => {
            app_state.clear_error();
        }
        KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
            let query = app_state.query.lines().join("");
            if query.trim().is_empty() {
                app_state.set_error("Cannot create note with empty query".to_string());
            } else {
                app_state.clear_error();
                let refresh_tx_clone = refresh_tx.clone();
                tokio::spawn(async move {
                    if let Ok(note_id) = data_sources::notes::create_note(&query, None).await {
                        let _ = data_sources::notes::open_note(&note_id).await;
                        refresh_tx_clone.try_send(()).ok();
                    }
                });
                app_state.query.delete_line_by_end();
                app_state.query.delete_line_by_head();
                app_state.filter_items();
            }
        }
        KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
            let selected_item = app_state.get_selected_item().cloned();
            match selected_item {
                Some(item) if item.handler == Handler::Note => {
                    app_state.clear_error();
                    let note_value = item.value.clone();
                    let refresh_tx_clone = refresh_tx.clone();
                    tokio::spawn(async move {
                        if data_sources::notes::delete_note(&note_value).await.is_ok() {
                            refresh_tx_clone.try_send(()).ok();
                        }
                    });
                }
                Some(_) => {
                    app_state.set_error("Can only delete notes with Ctrl+D".to_string());
                }
                None => {
                    app_state.set_error("No item selected to delete".to_string());
                }
            }
        }
        KeyCode::Tab => {
            let query = app_state.query.lines().join("");
            data_sources::web_search::open_chat_gpt(&query);
            return true;
        }
        KeyCode::Enter => {
            if let Some(item) = app_state.get_selected_item().cloned() {
                let item_for_exec = item.clone();
                let alt_modifier = key.modifiers == KeyModifiers::ALT;
                tokio::spawn(async move {
                    let _ = crate::commands::execute_command(&item_for_exec, alt_modifier).await;
                });
                let _ = history::add_to_history(&mut app_state.history, item);
                app_state.query.delete_line_by_end();
                app_state.query.delete_line_by_head();
                app_state.filter_items();
                // Reset selection to first item
                if !app_state.filtered_items.is_empty() {
                    app_state.table_state.select(Some(0));
                }
            } else {
                let query = app_state.query.lines().join("");
                if !query.is_empty() {
                    data_sources::web_search::search_web(&query);
                    app_state.query.delete_line_by_end();
                    app_state.query.delete_line_by_head();
                    app_state.filter_items();
                    // Reset selection to first item
                    if !app_state.filtered_items.is_empty() {
                        app_state.table_state.select(Some(0));
                    }
                }
            }
        }
        KeyCode::Down => {
            if !app_state.filtered_items.is_empty() {
                let i = match app_state.table_state.selected() {
                    Some(i) => (i + 1) % app_state.filtered_items.len(),
                    None => 0,
                };
                app_state.table_state.select(Some(i));
            }
        }
        KeyCode::Up => {
            if !app_state.filtered_items.is_empty() {
                let i = match app_state.table_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            app_state.filtered_items.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                app_state.table_state.select(Some(i));
            }
        }
        _ => {
            app_state.query.input(key);
            app_state.clear_error(); // Clear any errors when user starts typing
            app_state.filter_items(); // Filter static items immediately

            let query = app_state.query.lines().join("");
            
            // Only trigger searches for queries with 2+ characters
            if query.len() >= 2 {
                trigger_debounced_fs_search(query.clone(), fs_tx, error_tx.clone());
                trigger_debounced_web_search(query, web_tx);
            } else {
                // Clear items for short queries by sending empty vecs
                let _ = fs_tx.try_send(vec![]);
                let _ = web_tx.try_send(vec![]);
            }
        }
    }
    false // Do not exit
}

/// Trigger a debounced file system search that cancels previous searches
fn trigger_debounced_fs_search(query: String, fs_tx: mpsc::Sender<Vec<CommandItem>>, error_tx: Option<mpsc::Sender<String>>) {
    // Increment search generation to invalidate previous searches
    let current_generation = SEARCH_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    
    tokio::spawn(async move {
        // Debounce delay - wait for user to stop typing
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        
        // Check if this search is still the latest (not superseded by newer search)
        if SEARCH_GENERATION.load(Ordering::SeqCst) != current_generation {
            return; // This search was superseded, abort
        }
        
        // Perform the search with error handling
        let items = match data_sources::fs::spotlight_search_with_errors(&query, 8).await {
            Ok(items) => items,
            Err(error_msg) => {
                // Send error to UI error bar if channel is available
                if let Some(ref tx) = error_tx {
                    let _ = tx.send(error_msg).await;
                }
                vec![]
            }
        };
        
        // Double-check generation before sending results
        if SEARCH_GENERATION.load(Ordering::SeqCst) == current_generation {
            let _ = fs_tx.send(items).await;
        }
    });
}

/// Trigger a debounced web search that cancels previous searches
fn trigger_debounced_web_search(query: String, web_tx: mpsc::Sender<Vec<CommandItem>>) {
    // Increment search generation to invalidate previous searches
    let current_generation = WEB_SEARCH_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    
    tokio::spawn(async move {
        // Debounce delay for web search (responsive but not too aggressive)
        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        
        // Check if this search is still the latest
        if WEB_SEARCH_GENERATION.load(Ordering::SeqCst) != current_generation {
            return; // This search was superseded, abort
        }
        
        // Perform the web search
        if let Ok(suggestions) = data_sources::web_search::get_web_search_suggestions(query).await {
            // Double-check generation before sending results
            if WEB_SEARCH_GENERATION.load(Ordering::SeqCst) == current_generation {
                let _ = web_tx.send(suggestions).await;
            }
        }
    });
}
