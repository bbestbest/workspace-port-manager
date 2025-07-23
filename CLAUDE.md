## Project Memories

- This project creates a terminal-based TUI app like lazygit for macOS (Apple Silicon) using Rust and ratatui
- Purpose: Port management for development projects in ~/Workspace with recursive subdirectory scanning
- Full-screen terminal UI with bordered sections (header, project list, input panel, status bar)
- Detects working projects by config files (package.json, Cargo.toml, *.sln, etc.) not just folders
- Recursively scans ~/Workspace/mac/, ~/Workspace/agm/, ~/Workspace/azay/ subdirectories up to 3 levels deep
- Manages ports via .env files with PORT= entries
- Keyboard navigation: ↑/↓ navigate, Ctrl+U/Ctrl+D page up/down, r set port, R refresh, s sort, / search, e expand, w collapse, < top, > bottom, ENTER toggle, q quit, ESC cancel
- Color scheme: cyan header, yellow folder icons, green ports, dark gray selection
- Shows project relative paths like "mac/project1" with current port status or "─" if none
- Folder expand/collapse functionality with cursor can select both project items and folder headers
- Navigation includes folder headers - cursor can navigate to collapsed folder headers to expand them
- Refactored using SOLID principles with modular architecture:
  * project_scanner.rs - Single responsibility for finding projects
  * navigation.rs - Handles cursor movement and input state
  * ui.rs - Renders the terminal interface
  * port_manager.rs - Manages .env file operations
  * display_builder.rs - Converts projects to display items
  * main.rs - Coordinates modules and handles events
- Extensive documentation explaining Rust concepts for beginners (ownership, borrowing, pattern matching, error handling)
