mod cli;
mod commands;
mod core;
mod data_sources;
mod history;
mod icons;
mod input;
mod state;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use state::AppState;
use std::io::stdout;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    if let Some(search_command) = cli.search_command {
        return cli::run_search_command(search_command).await;
    }

    // TUI mode
    let (tx, mut rx) = mpsc::channel(1);
    let (fs_tx, mut fs_rx) = mpsc::channel(1);
    let (web_tx, mut web_rx) = mpsc::channel(1);
    let (refresh_tx, mut refresh_rx) = mpsc::channel(1);
    let (error_tx, mut error_rx) = mpsc::channel(1);

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let items = data_sources::get_all_items(false).await;
        tx_clone.send(items).await.ok();
    });

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(crossterm::terminal::SetTitle("Grinta"))?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let history = history::load_history()?;
    let initial_items = vec![];
    let mut app_state = AppState::new(history, initial_items);

    loop {
        let mut should_filter = false;
        
        if let Ok(items) = rx.try_recv() {
            app_state.items = items;
            should_filter = true;
        }

        if let Ok(items) = fs_rx.try_recv() {
            app_state.fs_items = items;
            should_filter = true;
        }

        if let Ok(items) = web_rx.try_recv() {
            app_state.web_items = items;
            should_filter = true;
        }

        if let Ok(error_msg) = error_rx.try_recv() {
            app_state.set_error(error_msg);
        }
        
        if should_filter {
            app_state.filter_items();
        }

        if refresh_rx.try_recv().is_ok() {
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let items = data_sources::get_all_items(false).await;
                tx_clone.send(items).await.ok();
            });
        }

        terminal.draw(|frame| ui::render(frame, &mut app_state))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if input::handle_key_event(
                        key,
                        &mut app_state,
                        fs_tx.clone(),
                        web_tx.clone(),
                        refresh_tx.clone(),
                        Some(error_tx.clone()),
                    ) {
                        break;
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
