use crate::state::AppState;
use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render(frame: &mut Frame, app_state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(frame.area());

    app_state
        .query
        .set_block(Block::default().borders(Borders::ALL).title("Search"));
    let input_widget = app_state.query.widget();
    frame.render_widget(input_widget, chunks[0]);

    let is_history_view = app_state.query.is_empty();
    let title = if is_history_view {
        "Recent Commands"
    } else {
        "Commands"
    };

    let rows: Vec<Row> = app_state
        .filtered_items
        .iter()
        .map(|item| {
            let icon_cell = Cell::from(item.icon.clone());
            let label_cell = Cell::from(item.label.clone());
            let context_cell = if is_history_view {
                if let Some(ran_at) = item.ran_at {
                    let now = Local::now();
                    if ran_at.date_naive() == now.date_naive() {
                        Cell::from(format!("Today {}", ran_at.format("%H:%M")))
                    } else {
                        Cell::from(ran_at.format("%b %d %H:%M").to_string())
                    }
                } else {
                    Cell::from("")
                }
            } else {
                Cell::from(item.handler.to_string())
            };
            Row::new(vec![icon_cell, label_cell, context_cell])
        })
        .collect();

    let constraints = [
        Constraint::Length(4),
        Constraint::Percentage(70),
        Constraint::Percentage(30),
    ];

    let table = Table::new(rows, constraints)
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(table, chunks[1], &mut app_state.table_state);
}
