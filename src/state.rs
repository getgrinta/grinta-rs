use crate::core::CommandItem;
use ratatui::widgets::TableState;
use tui_textarea::TextArea;

pub struct AppState<'a> {
    pub query: TextArea<'a>,
    pub items: Vec<CommandItem>,
    pub filtered_items: Vec<CommandItem>,
    pub table_state: TableState,
    pub history: Vec<CommandItem>,
    pub fs_items: Vec<CommandItem>,
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
        };
        state.filter_items();
        state
    }

    pub fn filter_items(&mut self) {
        let query = self.query.lines().join("\n");
        if query.is_empty() {
            self.filtered_items = self.history.clone();
            self.filtered_items.reverse();
        } else {
            let mut static_filtered: Vec<CommandItem> = self.items
                .iter()
                .filter(|item| {
                    item.label.to_lowercase().contains(&query.to_lowercase())
                        || item.value.to_lowercase().contains(&query.to_lowercase())
                })
                .cloned()
                .collect();

            let mut new_filtered = self.fs_items.clone();
            new_filtered.append(&mut static_filtered);
            self.filtered_items = new_filtered;
            self.filtered_items.sort_by_key(|item| item.handler);
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
}
