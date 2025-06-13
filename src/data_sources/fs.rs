use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use std::fs;


use tokio::task;

use crate::core::{CommandItem, Handler};

/// Debounce duration (milliseconds) before triggering Spotlight search.
/// This avoids firing too often while the user is typing.
const DEBOUNCE_MS: u64 = 300;

/// Create a `CommandItem` representing a file or folder found by Spotlight.
fn create_fs_command(path: &str) -> CommandItem {
    let metadata = fs::metadata(path);
    let is_dir = metadata.map(|m| m.is_dir()).unwrap_or(false);

    let label = if is_dir {
        PathBuf::from(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .to_string()
    } else {
        PathBuf::from(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .to_string()
    };

    let handler = if is_dir { Handler::Folder } else { Handler::File };

    let mut cmd = CommandItem::new(&label, handler, path);
    cmd.metadata.insert("type".to_string(), if is_dir { "folder" } else { "file" }.to_string());
    cmd
}

/// Perform a Spotlight (mdfind) search in the user's home directory for the given query.
/// Returns up to `max_results` `CommandItem`s asynchronously.
///
/// If the query is empty, returns an empty vector.
pub async fn spotlight_search(query: &str, max_results: usize) -> Vec<CommandItem> {
    if query.is_empty() {
        return vec![];
    }

    // Debounce lightly only for very short queries (to avoid firing every keystroke).
    // For longer queries we search immediately for better responsiveness.
    if query.len() <= 4 {
        tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)).await;
    }

    // Clone query for move into blocking task.
    let query_string = query.to_string();

    // Helper to avoid duplicating mdfind boilerplate
    fn run_mdfind(predicate: &str, dir: &PathBuf, lim: usize) -> Vec<String> {
        let output = Command::new("mdfind")
            .arg("-onlyin")
            .arg(dir)
            .arg(predicate)
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout.lines().take(lim).map(|s| s.to_string()).collect()
            }
            Err(e) => {
                eprintln!("Failed to run mdfind: {}", e);
                vec![]
            }
        }
    }

    // Run blocking Spotlight call in a separate thread so we don't block the async runtime.
    let results: Vec<CommandItem> = match task::spawn_blocking(move || {
        let home_path = match dirs::home_dir() {
            Some(p) => p,
            None => {
                eprintln!("Failed to determine home directory for Spotlight search.");
                return Vec::new(); // Cannot perform a home-directory-specific search
            }
        };

        // Fetch more items initially to allow for depth sorting before limiting to max_results.
        const PRE_SORT_FETCH_MULTIPLIER: usize = 3;
        const MAX_PRE_SORT_ITEMS: usize = 150; // Cap to prevent excessive mdfind calls
        
        let pre_sort_fetch_limit = if max_results == 0 { // Handle max_results = 0 explicitly
            0
        } else {
            max_results.saturating_mul(PRE_SORT_FETCH_MULTIPLIER).min(MAX_PRE_SORT_ITEMS)
        };

        if pre_sort_fetch_limit == 0 {
            return Vec::new();
        }

        // 1. Exact display name match
        let exact_predicate = format!("kMDItemDisplayName == '{}'cd", query_string);
        let mut paths = run_mdfind(&exact_predicate, &home_path, pre_sort_fetch_limit);

        // 2. Broader "starts with" match if we haven't reached the pre_sort_fetch_limit
        if paths.len() < pre_sort_fetch_limit {
            let fuzzy_predicate = format!("kMDItemDisplayName == '{}*'cd", query_string);
            let needed = pre_sort_fetch_limit - paths.len();
            if needed > 0 { // Ensure 'needed' is positive before running mdfind
                let mut fuzzy_paths = run_mdfind(&fuzzy_predicate, &home_path, needed);

                // Deduplicate while preserving order (exact matches first)
                fuzzy_paths.retain(|p| !paths.contains(p));
                paths.extend(fuzzy_paths);
            }
        }

        // Sort all collected paths by depth relative to home_path (shallowest first)
        // Uses sort_by_cached_key for efficiency if depth calculation is non-trivial.
        paths.sort_by_cached_key(|p_str| {
            let path_obj = PathBuf::from(p_str);
            path_obj.strip_prefix(&home_path)
                .map_or(usize::MAX, |rel_path| rel_path.components().count())
            // usize::MAX ensures paths not relative to home (unexpected) go to the end.
        });

        // Now, take the top `max_results` and map to CommandItem
        paths
            .into_iter()
            .take(max_results)
            .map(|p| create_fs_command(&p))
            .collect::<Vec<_>>()
    })
    .await
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Task join error in spotlight_search: {}", e);
            vec![]
        }
    };

    results
}
