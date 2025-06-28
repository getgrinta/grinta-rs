# grinta-rs

**Grinta is a blazing-fast, AI-powered Spotlight replacement for macOS, built with Rust.**

This project is the core backend for the Grinta desktop application, designed to be a powerful and extensible search tool that helps you find what you need in an instant. Whether you're looking for an application, a file, a browser bookmark, or even an Apple Note, Grinta has you covered.

## Features

- **Application Launcher**: Quickly find and launch any application on your system.
- **File & Folder Search**: Instantly search for files and folders within your home directory.
- **Browser Bookmarks**: Access your Chrome and Chromium bookmarks on the fly.
- **Apple Notes Integration**: Seamlessly search and open your Apple Notes.
- **Apple Shortcuts**: List and run your Apple Shortcuts directly from Grinta.
- **Web Search**: Perform web searches and get instant suggestions from Startpage.
- **AI-Powered Search**: Grinta uses AI to provide you with the most relevant search results.
- **Command-Line Interface**: A powerful CLI for scripting and advanced users.
- **TUI Mode**: A friendly and intuitive terminal user interface.

## How to Use

Grinta can be used in two modes: as a Terminal User Interface (TUI) or as a Command-Line Interface (CLI).

### TUI Mode

To launch the TUI, simply run:

```bash
grinta
```

This will open an interactive search prompt where you can start typing to see instant results.

### CLI Mode

For scripting or quick searches, you can use the `search` subcommand:

```bash
grinta search "my query"
```

This will output the search results in JSON format, which you can then pipe to other tools like `jq` for further processing.

## Data Sources

Grinta aggregates data from multiple sources to provide comprehensive search results:

- **Applications**: All `.app` files in your `/Applications` and `~/Applications` directories.
- **Files & Folders**: Your user's home directory (`$HOME`).
- **Browser Bookmarks**: Chrome and Chromium.
- **Apple Notes**: Your local Apple Notes.
- **Apple Shortcuts**: Your saved Apple Shortcuts.

## Key bindings

- **Arrow Up**: Previous item.
- **Arrow Down**: Next item.
- **Esc/Ctrl+c**: Exit.
- **Tab**: AI query.
- **Alt+Enter**: Highlight file/directory in Finder.

## Tech Stack

- [**Tokio**](https://tokio.rs/) - For asynchronous operations.
- [**Ratatui**](https://ratatui.rs/) - To create the Terminal User Interface.
- [**Clap**](https://crates.io/crates/clap) - For parsing command-line arguments.
