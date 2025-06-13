use crate::data_sources;
use anyhow::Result;
use open;
use std::process::Command;

pub use crate::core::{CommandItem, Handler};

pub fn execute_command(item: &CommandItem, alt_modifier_active: bool) -> Result<()> {
    match item.handler {
        Handler::Url => {
            open::that(&item.value)?;
        }
        Handler::App => {
            #[cfg(target_os = "macos")]
            {
                Command::new("open").arg(&item.value).spawn()?;
            }
            #[cfg(not(target_os = "macos"))]
            {
                open::that(&item.value)?;
            }
        }
        Handler::Note => {
            data_sources::notes::open_note(&item.value)?;
        }
        Handler::File | Handler::Folder => {
            if alt_modifier_active {
                #[cfg(target_os = "macos")]
                {
                    Command::new("open").arg("-R").arg(&item.value).spawn()?;
                }
                #[cfg(not(target_os = "macos"))]
                {
                    // For non-macOS, Alt+Enter on a file/folder could potentially
                    // open its parent directory. This is a placeholder for that logic.
                    // For now, it will just open the item directly.
                    open::that(&item.value)?;
                }
            } else {
                open::that(&item.value)?;
            }
        }
        Handler::Automation => {
            #[cfg(target_os = "macos")]
            {
                Command::new("shortcuts").args(["run", &item.value]).spawn()?;
            }
        }
    }
    Ok(())
}
