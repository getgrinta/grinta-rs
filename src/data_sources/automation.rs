use tokio::process::Command;

use crate::core::{CommandItem, CommandType, Handler};

/// Retrieve all macOS Shortcuts available to the current user.
/// This is performed by invoking the `shortcuts list` CLI (macOS 12+).
/// Returns a vector of `CommandItem`s that can be displayed in the UI.
#[cfg(target_os = "macos")]
pub async fn get_shortcuts() -> Vec<CommandItem> {
    let output = match Command::new("shortcuts").arg("list").output().await {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Failed to execute `shortcuts` command: {}", e);
            return vec![];
        }
    };

    if !output.status.success() {
        eprintln!(
            "Failed to list shortcuts: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return vec![];
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| {
            let mut cmd = CommandItem::new(line, Handler::Automation, line);
            cmd.metadata.insert("type".to_string(), "shortcut".to_string());
            cmd.kind = CommandType::App; // treat as app-like
            cmd
        })
        .collect()
}

/// Stub implementation for non-macOS targets.
#[cfg(not(target_os = "macos"))]
pub async fn get_shortcuts() -> Vec<CommandItem> {
    Vec::new()
}
