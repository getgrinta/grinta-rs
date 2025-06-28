use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncReadExt;

use crate::core::{CommandItem, Handler};

#[derive(Debug, Deserialize, Serialize)]
struct BookmarkNode {
    name: Option<String>,
    #[serde(rename = "type")]
    node_type: Option<String>,
    url: Option<String>,
    children: Option<Vec<BookmarkNode>>,
    date_added: Option<String>,
    date_last_used: Option<String>,
    guid: Option<String>,
    id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct BookmarkRoots {
    bookmark_bar: BookmarkNode,
    other: BookmarkNode,
    synced: BookmarkNode,
}

#[derive(Debug, Deserialize, Serialize)]
struct BookmarkFile {
    roots: BookmarkRoots,
    version: u32,
}

/// Get all bookmarks from Chrome and Chromium browsers
pub async fn get_browser_bookmarks() -> Vec<CommandItem> {
    let mut bookmarks = Vec::new();
    
    // Get Chrome bookmarks
    bookmarks.extend(get_chrome_bookmarks().await);
    
    // Get Chromium bookmarks
    bookmarks.extend(get_chromium_bookmarks().await);
    
    bookmarks
}

/// Get bookmarks from Chrome browser
async fn get_chrome_bookmarks() -> Vec<CommandItem> {
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let base_path = home_dir.join("Library/Application Support/Google/Chrome");
    
    // Check Default profile
    let mut bookmarks = get_bookmarks_from_profile(&base_path.join("Default")).await;
    
    // Check numbered profiles (1-9)
    for i in 1..=9 {
        let profile_path = base_path.join(format!("Profile {}", i));
        bookmarks.extend(get_bookmarks_from_profile(&profile_path).await);
    }
    
    bookmarks
}

/// Get bookmarks from Chromium browser
async fn get_chromium_bookmarks() -> Vec<CommandItem> {
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let base_path = home_dir.join("Library/Application Support/Chromium");
    
    // Check Default profile
    let mut bookmarks = get_bookmarks_from_profile(&base_path.join("Default")).await;
    
    // Check numbered profiles (1-9)
    for i in 1..=9 {
        let profile_path = base_path.join(format!("Profile {}", i));
        bookmarks.extend(get_bookmarks_from_profile(&profile_path).await);
    }
    
    bookmarks
}

/// Get bookmarks from a specific browser profile
async fn get_bookmarks_from_profile(profile_path: &Path) -> Vec<CommandItem> {
    let bookmarks_path = profile_path.join("Bookmarks");
    
    if !bookmarks_path.exists() {
        return Vec::new();
    }
    
    match read_bookmarks_file(&bookmarks_path).await {
        Ok(bookmark_file) => extract_bookmarks_from_file(bookmark_file),
        Err(e) => {
            eprintln!("Error reading bookmarks from {:?}: {}", bookmarks_path, e);
            Vec::new()
        }
    }
}

/// Read and parse the bookmarks file
async fn read_bookmarks_file(path: &Path) -> Result<BookmarkFile, Box<dyn std::error::Error + Send + Sync>> {
    let mut file = fs::File::open(path).await?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;
    
    let bookmark_file: BookmarkFile = serde_json::from_str(&contents)?;
    Ok(bookmark_file)
}

/// Extract all bookmarks from the bookmark file structure
fn extract_bookmarks_from_file(bookmark_file: BookmarkFile) -> Vec<CommandItem> {
    let mut bookmarks = Vec::new();
    
    // Process bookmark bar
    process_bookmark_node(&bookmark_file.roots.bookmark_bar, &mut bookmarks);
    
    // Process other bookmarks
    process_bookmark_node(&bookmark_file.roots.other, &mut bookmarks);
    
    // Process synced bookmarks
    process_bookmark_node(&bookmark_file.roots.synced, &mut bookmarks);
    
    bookmarks
}

/// Recursively process a bookmark node and extract all bookmarks
fn process_bookmark_node(node: &BookmarkNode, bookmarks: &mut Vec<CommandItem>) {
    // If this is a URL bookmark, add it to the list
    if let (Some(name), Some(url), Some(node_type)) = (&node.name, &node.url, &node.node_type) {
        if node_type == "url" {
            bookmarks.push(CommandItem::new(format!("{} (Bookmark)", name).as_str(), Handler::Url, url));
        }
    }
    
    // Recursively process children
    if let Some(children) = &node.children {
        for child in children {
            process_bookmark_node(child, bookmarks);
        }
    }
}
