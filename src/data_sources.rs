pub mod automation;
pub mod bookmarks;
pub mod fs;
pub mod notes;
pub mod web_search;

use crate::core::{CommandItem, Handler};
use crate::icons;

pub async fn get_all_items(extract_icons: bool) -> Vec<CommandItem> {
    let mut items = Vec::new();

    #[cfg(target_os = "macos")]
    {
        items.extend(get_macos_applications(extract_icons));
        items.extend(notes::get_notes());
        items.extend(bookmarks::get_browser_bookmarks());
        items.extend(automation::get_shortcuts());
    }
    
    items
}

#[cfg(target_os = "macos")]
fn get_macos_applications(extract_icons: bool) -> Vec<CommandItem> {
        let applications_dirs = vec!["/Applications", "/System/Applications", "/System/Applications/Utilities"];
    let mut apps = Vec::new();

    for dir in applications_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("app") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        let path_str = path.to_str().unwrap_or("");
                        let mut item = CommandItem::new(name, Handler::App, path_str);
                        if extract_icons {
                            item.base64_icon = icons::extract_app_icon(path_str);
                        }
                        apps.push(item);
                    }
                }
            }
        }
    }
    apps
}
