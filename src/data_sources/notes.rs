use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::core::{CommandItem, Handler};

#[derive(Debug, Deserialize, Serialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub folder: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[cfg(target_os = "macos")]
pub async fn get_notes() -> Vec<CommandItem> {
    let mut notes = Vec::new();
    
    // JavaScript to fetch notes from the Notes app
    let script = r#"
        const Notes = Application("Notes");
        Notes.includeStandardAdditions = true;

        const folders = Notes.folders();
        const notes = [];

        folders.forEach(function(folder) {
            return folder.notes().forEach(function(note) {
                notes.push({
                    id: note.id(),
                    title: note.name(),
                    folder: folder.name(),
                    createdAt: note.creationDate(),
                    updatedAt: note.modificationDate()
                })
            });
        });
        console.log(JSON.stringify(notes));
    "#;
    
    // Run osascript to execute the JavaScript
    if let Ok(output) = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", script])
        .output()
        .await
    {
        // According to the TypeScript reference, the output is in stderr, not stdout
        if let Ok(output_str) = String::from_utf8(output.stderr) {
            // Parse the JSON output
            if let Ok(parsed_notes) = serde_json::from_str::<Vec<Note>>(&output_str) {
                for note in parsed_notes {
                    // Create a command item for each note
                    // Store the note ID in the value field
                    let label = format!("{} ({})", note.title, note.folder);
                    notes.push(CommandItem::new(&label, Handler::Note, &note.id));
                }
            }
        }
    }
    
    notes
}

#[cfg(target_os = "macos")]
pub async fn open_note(note_id: &str) -> std::io::Result<()> {
    // Open the note with its ID using AppleScript
    // Using the simpler and more reliable approach from the TypeScript reference
    let script = format!(r#"
        const Notes = Application("Notes");
        Notes.includeStandardAdditions = true;
        const note = Notes.notes.byId("{}");
        Notes.activate();
        Notes.show(note);
    "#, note_id);
    
    Command::new("osascript")
        .args(["-l", "JavaScript", "-e", &script])
        .output()
        .await
        .map(|_| ())
}

#[cfg(target_os = "macos")]
pub async fn create_note(name: &str, body: Option<&str>) -> std::io::Result<String> {
    // Format the note body with title
    let formatted_body = format_note_body(name, body.unwrap_or(""));
    
    // JavaScript to create a new note
    let script = format!(r#"
        const Notes = Application("Notes");
        Notes.includeStandardAdditions = true;
        const accountName = "iCloud";
        const folderName = "Notes";
        const account = Notes.accounts.byName(accountName);
        const folder = account.folders.byName(folderName);
        const newNote = Notes.Note({{  
            body: `{}`
        }});
        folder.notes.push(newNote);
        const noteId = newNote.id().trim();
        console.log(noteId);
    "#, formatted_body);
    
    // Run osascript to execute the JavaScript
    let output = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", &script])
        .output()
        .await?;
    
    // Get the note ID from stderr
    if let Ok(note_id) = String::from_utf8(output.stderr) {
        // Remove any newlines
        let note_id = note_id.trim().to_string();
        Ok(note_id)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get note ID"
        ))
    }
}

#[cfg(target_os = "macos")]
pub async fn delete_note(note_id: &str) -> std::io::Result<()> {
    // JavaScript to delete a note
    let script = format!(r#"
        const Notes = Application("Notes");
        Notes.includeStandardAdditions = true;
        const note = Notes.notes.byId("{}");
        note.delete();
    "#, note_id);
    
    Command::new("osascript")
        .args(["-l", "JavaScript", "-e", &script])
        .output()
        .await
        .map(|_| ())
}

// Helper function to format note body with title
#[cfg(target_os = "macos")]
fn format_note_body(title: &str, body: &str) -> String {
    let title_template = format!("<div><h1>{}</h1></div>", title);
    if body.is_empty() {
        return title_template;
    }
    format!("{}
<div>{}</div>", title_template, body)
}

/// Stub implementation for non-macOS targets.
#[cfg(not(target_os = "macos"))]
pub async fn get_notes() -> Vec<CommandItem> {
    Vec::new()
}
