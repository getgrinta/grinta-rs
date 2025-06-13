use crate::core::CommandItem;
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

const HISTORY_FILE: &str = "grinta_history.json";

fn history_file_path() -> Result<PathBuf> {
    let mut path = dirs::data_dir().context("Failed to get data directory")?;
    path.push("grinta-rs");
    fs::create_dir_all(&path)?;
    path.push(HISTORY_FILE);
    Ok(path)
}

pub fn load_history() -> Result<Vec<CommandItem>> {
    let path = history_file_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let history = serde_json::from_str(&contents).unwrap_or_else(|_| Vec::new());
    Ok(history)
}

pub fn save_history(history: &[CommandItem]) -> Result<()> {
    let path = history_file_path()?;
    let mut file = File::create(path)?;
    let json = serde_json::to_string_pretty(history)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

pub fn add_to_history(history: &mut Vec<CommandItem>, mut item: CommandItem) -> Result<()> {
    item.mark_executed();
    
    history.retain(|h| h.label != item.label || h.handler != item.handler || h.value != item.value);
    history.push(item);

    save_history(history)
}
