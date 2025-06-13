use crate::{
    core::{CommandItem, Handler},
    data_sources,
    history,
    state::AppState,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

pub fn handle_key_event(
    key: KeyEvent,
    app_state: &mut AppState,
    fs_tx: mpsc::Sender<Vec<CommandItem>>,
    refresh_tx: mpsc::Sender<()>, 
) -> bool {
    match key.code {
        KeyCode::Esc => return true, // Signal to exit
        KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => return true,
        KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
            let query = app_state.query.lines().join("");
            if let Ok(note_id) = data_sources::notes::create_note(&query, None) {
                let label = if query.is_empty() {
                    "Untitled Note".to_string()
                } else {
                    query
                };
                let new_note_item = CommandItem::new(&label, Handler::Note, &note_id);
                let _ = history::add_to_history(&mut app_state.history, new_note_item);

                data_sources::notes::open_note(&note_id).ok();
                app_state.query.delete_line_by_end();
                app_state.query.delete_line_by_head();
                refresh_tx.try_send(()).ok();
                app_state.filter_items();
            }
        }
        KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
            if let Some(item) = app_state.get_selected_item() {
                if item.handler == Handler::Note {
                    if data_sources::notes::delete_note(&item.value).is_ok() {
                        refresh_tx.try_send(()).ok();
                    }
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
                if crate::commands::execute_command(&item, key.modifiers == KeyModifiers::ALT).is_ok() {
                    let _ = history::add_to_history(&mut app_state.history, item);
                    app_state.query.delete_line_by_end();
                    app_state.query.delete_line_by_head();
                    app_state.filter_items();
                }
            } else {
                let query = app_state.query.lines().join("");
                if !query.is_empty() {
                    data_sources::web_search::search_web(&query);
                    let search_command = data_sources::web_search::create_web_search_command(&query);
                    let _ = history::add_to_history(&mut app_state.history, search_command);
                    app_state.query.delete_line_by_end();
                    app_state.query.delete_line_by_head();
                    app_state.filter_items();
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
            app_state.filter_items(); // Filter static items immediately

            let query = app_state.query.lines().join("");
            let tx = fs_tx.clone();
            tokio::spawn(async move {
                let items = data_sources::fs::spotlight_search(&query, 10).await;
                tx.send(items).await.ok();
            });
        }
    }
    false // Do not exit
}
