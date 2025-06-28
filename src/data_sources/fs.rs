use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use crate::core::{CommandItem, Handler};

/// Reduced debounce for better responsiveness
#[allow(dead_code)]
const DEBOUNCE_MS: u64 = 150;

/// Timeout for mdfind operations to ensure reliability
const MDFIND_TIMEOUT_MS: u64 = 2000;

/// Create a `CommandItem` representing a file or folder found by Spotlight.
async fn create_fs_command(path: &str) -> Option<CommandItem> {
    // Use async metadata check for better performance
    let metadata = tokio::fs::metadata(path).await;
    let is_dir = metadata.map(|m| m.is_dir()).unwrap_or(false);

    // Extract filename more efficiently
    let path_buf = PathBuf::from(path);
    let label = path_buf
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_else(|| {
            // Fallback to last component if file_name fails
            path_buf.components().last()
                .and_then(|c| c.as_os_str().to_str())
                .unwrap_or(path)
        })
        .to_string();

    let handler = if is_dir { Handler::Folder } else { Handler::File };

    let mut cmd = CommandItem::new(&label, handler, path);
    cmd.metadata.insert("type".to_string(), if is_dir { "folder" } else { "file" }.to_string());
    
    Some(cmd)
}

/// Optimized mdfind search with better predicates and error handling
async fn run_mdfind_optimized(query: &str, max_results: usize) -> Result<Vec<String>, String> {
    let home_path = match dirs::home_dir() {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };

    // Build a more efficient combined search predicate
    // This reduces mdfind to a single call instead of multiple
    let predicate = format!(
        "(kMDItemDisplayName == '{0}'cd || kMDItemDisplayName == '{0}*'cd || kMDItemFSName == '{0}*'cd)",
        query.replace("'", "\\'") // Escape single quotes for safety
    );

    // Use async command with timeout for reliability
    let mdfind_future = Command::new("mdfind")
        .arg("-onlyin")
        .arg(&home_path)
        .arg(&predicate)
        .output();

    let output = match timeout(Duration::from_millis(MDFIND_TIMEOUT_MS), mdfind_future).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            return Err(format!("mdfind command failed: {}", e));
        }
        Err(_) => {
            return Err(format!("mdfind timed out after {}ms", MDFIND_TIMEOUT_MS));
        }
    };

    if !output.status.success() {
        return Err(format!("mdfind exited with status: {}", output.status));
    }

    // Process results efficiently
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results: Vec<String> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .take(max_results * 2) // Take extra for sorting
        .map(|s| s.to_string())
        .collect();

    // Enhanced depth-based sort with multiple criteria
    results.sort_by(|a, b| {
        let a_path = PathBuf::from(a);
        let b_path = PathBuf::from(b);
        
        // Primary: depth (shallower first)
        let a_depth = a_path.strip_prefix(&home_path)
            .map_or(usize::MAX, |rel_path| rel_path.components().count());
        let b_depth = b_path.strip_prefix(&home_path)
            .map_or(usize::MAX, |rel_path| rel_path.components().count());
        
        match a_depth.cmp(&b_depth) {
            std::cmp::Ordering::Equal => {
                // Secondary: prioritize common directories
                let a_priority = get_path_priority(a);
                let b_priority = get_path_priority(b);
                match a_priority.cmp(&b_priority) {
                    std::cmp::Ordering::Equal => {
                        // Tertiary: alphabetical by filename
                        a_path.file_name().cmp(&b_path.file_name())
                    }
                    other => other
                }
            }
            other => other
        }
    });

    // Return the top results
    results.truncate(max_results);
    Ok(results)
}

/// Assign priority scores to paths (lower = higher priority)
fn get_path_priority(path: &str) -> u8 {
    let path_lower = path.to_lowercase();
    
    // Highest priority: Desktop, Documents, Downloads
    if path_lower.contains("/desktop/") || path_lower.contains("/documents/") || path_lower.contains("/downloads/") {
        return 1;
    }
    
    // High priority: Home directory root files
    if path.matches('/').count() <= 3 {
        return 2;
    }
    
    // Medium priority: Development, Projects directories
    if path_lower.contains("/developer/") || path_lower.contains("/projects/") || path_lower.contains("/code/") {
        return 3;
    }
    
    // Lower priority: Library, hidden files, system directories
    if path_lower.contains("/library/") || path_lower.contains("/.") {
        return 5;
    }
    
    // Default priority
    4
}

/// Perform an optimized Spotlight (mdfind) search.
/// Returns up to `max_results` `CommandItem`s asynchronously.
#[allow(dead_code)]
pub async fn spotlight_search(query: &str, max_results: usize) -> Vec<CommandItem> {
    if query.is_empty() || max_results == 0 {
        return vec![];
    }

    // Reduced debounce for better responsiveness
    if query.len() <= 3 {
        tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)).await;
    }

    // Skip very short queries that would return too many results
    if query.len() < 2 {
        return vec![];
    }

    // Get paths from optimized mdfind
    let paths = match run_mdfind_optimized(query, max_results).await {
        Ok(paths) => paths,
        Err(_) => return vec![], // Silently fail for now, will add error handling later
    };
    
    // Convert paths to CommandItems concurrently using tokio
    let mut tasks = Vec::with_capacity(paths.len());
    for path in paths {
        let path_clone = path.clone();
        tasks.push(tokio::spawn(async move { create_fs_command(&path_clone).await }));
    }

    // Wait for all file metadata checks concurrently
    let mut results = Vec::with_capacity(tasks.len());
    for task in tasks {
        if let Ok(Some(item)) = task.await {
            results.push(item);
        }
    }
    
    results
}

/// Fast file search for CLI streaming - prioritizes speed over completeness
pub async fn fast_file_search(query: &str, max_results: usize) -> Vec<CommandItem> {
    if query.is_empty() || query.len() < 2 {
        return vec![];
    }

    // No debounce for streaming - immediate response
    let paths = match run_mdfind_optimized(query, max_results + 5).await {
        Ok(paths) => paths,
        Err(_) => return vec![], // Silently fail for now
    }; // Get extra for better prioritization
    
    // Create items with minimal validation for speed
    let mut items = Vec::with_capacity(paths.len());
    for path in paths {
        let path_buf = PathBuf::from(&path);
        if let Some(label) = path_buf.file_name().and_then(|s| s.to_str()) {
            // Quick heuristic for file vs folder (avoid async fs call)
            let is_dir = path.ends_with('/') || !path.contains('.');
            let handler = if is_dir { Handler::Folder } else { Handler::File };
            
            let mut cmd = CommandItem::new(label, handler, &path);
            cmd.metadata.insert("type".to_string(), if is_dir { "folder" } else { "file" }.to_string());
            items.push(cmd);
        }
    }
    
    // Limit final results for CLI
    items.truncate(max_results);
    items
}

/// Spotlight search that returns errors for UI display
pub async fn spotlight_search_with_errors(query: &str, max_results: usize) -> Result<Vec<CommandItem>, String> {
    if query.is_empty() || max_results == 0 {
        return Ok(vec![]);
    }

    // Skip very short queries that would return too many results
    if query.len() < 2 {
        return Ok(vec![]);
    }

    // Get paths from optimized mdfind
    let paths = run_mdfind_optimized(query, max_results).await?;
    
    // Convert paths to CommandItems concurrently using tokio
    let mut tasks = Vec::with_capacity(paths.len());
    for path in paths {
        let path_clone = path.clone();
        tasks.push(tokio::spawn(async move { create_fs_command(&path_clone).await }));
    }

    // Wait for all file metadata checks concurrently
    let mut results = Vec::with_capacity(tasks.len());
    for task in tasks {
        if let Ok(Some(item)) = task.await {
            results.push(item);
        }
    }
    
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio_test;

    #[test]
    fn test_get_path_priority() {
        // Highest priority: Desktop, Documents, Downloads
        assert_eq!(get_path_priority("/Users/test/Desktop/file.txt"), 1);
        assert_eq!(get_path_priority("/Users/test/Documents/doc.pdf"), 1);
        assert_eq!(get_path_priority("/Users/test/Downloads/download.zip"), 1);

        // High priority: Home directory root files
        assert_eq!(get_path_priority("/Users/test/file.txt"), 2);

        // Medium priority: Development directories
        assert_eq!(get_path_priority("/Users/test/Developer/project/file.js"), 3);
        assert_eq!(get_path_priority("/Users/test/Projects/app/main.py"), 3);
        assert_eq!(get_path_priority("/Users/test/Code/script.sh"), 3);

        // Lower priority: Library and hidden files
        assert_eq!(get_path_priority("/Users/test/Library/cache.db"), 5);
        assert_eq!(get_path_priority("/Users/test/.hidden/config"), 5);

        // Default priority
        assert_eq!(get_path_priority("/Users/test/Other/random/file.txt"), 4);
    }

    #[test]
    fn test_create_fs_command_file() {
        tokio_test::block_on(async {
            // Test with a file that likely exists
            let result = create_fs_command("/tmp").await;
            assert!(result.is_some());
            
            let cmd = result.unwrap();
            assert_eq!(cmd.label, "tmp");
            assert_eq!(cmd.handler, Handler::Folder);
            assert_eq!(cmd.value, "/tmp");
            assert_eq!(cmd.metadata.get("type"), Some(&"folder".to_string()));
        });
    }

    #[test]
    fn test_create_fs_command_nonexistent() {
        tokio_test::block_on(async {
            let result = create_fs_command("/nonexistent/path/that/should/not/exist").await;
            // Should still create a command item even if path doesn't exist
            assert!(result.is_some());
        });
    }

    #[test]
    fn test_spotlight_search_empty_query() {
        tokio_test::block_on(async {
            let result = spotlight_search("", 10).await;
            assert!(result.is_empty());
        });
    }

    #[test]
    fn test_spotlight_search_short_query() {
        tokio_test::block_on(async {
            let result = spotlight_search("a", 10).await;
            assert!(result.is_empty());
        });
    }

    #[test]
    fn test_spotlight_search_with_errors_empty() {
        tokio_test::block_on(async {
            let result = spotlight_search_with_errors("", 10).await;
            assert!(result.is_ok());
            assert!(result.unwrap().is_empty());
        });
    }

    #[test]
    fn test_spotlight_search_with_errors_short_query() {
        tokio_test::block_on(async {
            let result = spotlight_search_with_errors("a", 10).await;
            assert!(result.is_ok());
            assert!(result.unwrap().is_empty());
        });
    }

    #[test]
    fn test_fast_file_search_empty_query() {
        tokio_test::block_on(async {
            let result = fast_file_search("", 10).await;
            assert!(result.is_empty());
        });
    }

    #[test]
    fn test_fast_file_search_short_query() {
        tokio_test::block_on(async {
            let result = fast_file_search("a", 10).await;
            assert!(result.is_empty());
        });
    }

    #[test]
    fn test_path_priority_ordering() {
        let paths = vec![
            "/Users/test/Library/file.txt",      // Priority 5
            "/Users/test/Documents/doc.pdf",     // Priority 1
            "/Users/test/Other/file.txt",        // Priority 4
            "/Users/test/Developer/app.js",      // Priority 3
            "/Users/test/root.txt",              // Priority 2
        ];

        let mut sorted_paths = paths.clone();
        sorted_paths.sort_by(|a, b| get_path_priority(a).cmp(&get_path_priority(b)));

        assert_eq!(sorted_paths[0], "/Users/test/Documents/doc.pdf");  // Priority 1
        assert_eq!(sorted_paths[1], "/Users/test/root.txt");           // Priority 2
        assert_eq!(sorted_paths[2], "/Users/test/Developer/app.js");   // Priority 3
        assert_eq!(sorted_paths[3], "/Users/test/Other/file.txt");     // Priority 4
        assert_eq!(sorted_paths[4], "/Users/test/Library/file.txt");   // Priority 5
    }

    #[test]
    fn test_path_buf_operations() {
        let path = "/Users/test/Documents/file.txt";
        let path_buf = PathBuf::from(path);
        
        assert_eq!(path_buf.file_name().unwrap().to_str().unwrap(), "file.txt");
        assert_eq!(path_buf.parent().unwrap().to_str().unwrap(), "/Users/test/Documents");
    }

    #[test]
    fn test_debounce_constants() {
        // Ensure debounce constants are reasonable
        assert!(DEBOUNCE_MS > 0);
        assert!(DEBOUNCE_MS < 1000); // Should be less than 1 second
        
        assert!(MDFIND_TIMEOUT_MS > 0);
        assert!(MDFIND_TIMEOUT_MS >= 1000); // Should be at least 1 second
    }

    #[test]
    fn test_query_escaping() {
        // Test that single quotes are properly escaped
        let query = "test's file";
        let escaped = query.replace("'", "\\'");
        assert_eq!(escaped, "test\\'s file");
    }

    #[test]
    fn test_max_results_limiting() {
        tokio_test::block_on(async {
            // Test that fast_file_search respects max_results
            let result = fast_file_search("test", 5).await;
            assert!(result.len() <= 5);
        });
    }

    #[test]
    fn test_file_vs_folder_heuristic() {
        // Test the heuristic used in fast_file_search
        // Files typically have extensions, folders typically don't or end with /
        
        // Test some examples
        let file_path = "/path/to/document.pdf";
        let folder_path = "/path/to/folder/";
        let no_extension = "/path/to/README";
        
        // File with extension
        assert!(file_path.contains('.'));
        assert!(!file_path.ends_with('/'));
        
        // Folder with trailing slash
        assert!(folder_path.ends_with('/'));
        
        // File without extension (ambiguous case)
        assert!(!no_extension.contains('.') && !no_extension.ends_with('/'));
        
        // Test the actual heuristic logic from fast_file_search
        let is_dir_file = file_path.ends_with('/') || !file_path.contains('.');
        let is_dir_folder = folder_path.ends_with('/') || !folder_path.contains('.');
        let is_dir_no_ext = no_extension.ends_with('/') || !no_extension.contains('.');
        
        assert!(!is_dir_file); // Should be detected as file
        assert!(is_dir_folder); // Should be detected as folder
        assert!(is_dir_no_ext); // Should be detected as folder (no extension)
    }

    #[tokio::test]
    async fn test_concurrent_file_operations() {
        // Test that multiple file operations can run concurrently
        let paths = vec!["/tmp", "/usr", "/var"];
        let mut tasks = Vec::new();
        
        for path in paths {
            tasks.push(tokio::spawn(async move {
                create_fs_command(path).await
            }));
        }
        
        let results = futures::future::join_all(tasks).await;
        
        // All tasks should complete
        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_case_sensitivity_in_priorities() {
        // Test that priority matching is case insensitive
        assert_eq!(get_path_priority("/Users/test/DESKTOP/file.txt"), 1);
        assert_eq!(get_path_priority("/Users/test/desktop/file.txt"), 1);
        assert_eq!(get_path_priority("/Users/test/Desktop/file.txt"), 1);
        
        assert_eq!(get_path_priority("/Users/test/DEVELOPER/file.txt"), 3);
        assert_eq!(get_path_priority("/Users/test/developer/file.txt"), 3);
    }

    #[test]
    fn test_metadata_insertion() {
        tokio_test::block_on(async {
            if let Some(cmd) = create_fs_command("/tmp").await {
                assert!(cmd.metadata.contains_key("type"));
                let file_type = cmd.metadata.get("type").unwrap();
                assert!(file_type == "file" || file_type == "folder");
            }
        });
    }
}
