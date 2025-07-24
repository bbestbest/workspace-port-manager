/*!
 * Navigation Module
 * 
 * This module follows the Single Responsibility Principle (SRP).
 * It has ONE job: handling cursor navigation and list state management.
 * 
 * Key Rust concepts demonstrated:
 * - Enums: Types that can be one of several variants (like union types)
 * - Pattern matching: Rust's powerful way of handling different cases
 * - Borrowing: Using references (&) to avoid taking ownership
 * - Mutable references (&mut): Allowing modification through references
 * - Method chaining: Calling methods one after another
 */

use crate::project_scanner::ProjectInfo;
use std::collections::HashSet;

/// Represents different input modes the application can be in
/// 
/// Enums in Rust are powerful - each variant can have different data.
/// This is similar to discriminated unions in TypeScript.
#[derive(Debug, PartialEq)]
pub enum InputMode {
    /// Normal navigation mode - user can move cursor and trigger actions
    Normal,
    /// Editing mode - user is typing a port number
    Editing,
    /// Searching mode - user is typing a search query
    Searching,
}

/// Represents an item in the display list
/// 
/// This separates the display logic from the data logic.
/// One ProjectInfo might generate multiple DisplayItems.
#[derive(Debug, Clone)]
pub struct DisplayItem {
    /// The text content to show
    pub content: String,
    /// Whether this is a folder header (true) or project item (false)
    pub is_header: bool,
    /// Index into the original projects array (None for headers)
    pub project_index: Option<usize>,
}

/// Navigation Handler - manages cursor movement and list state
/// 
/// This struct follows SRP: it only handles navigation logic.
/// It doesn't know about UI rendering or project scanning.
pub struct NavigationHandler {
    /// Currently selected index in the display list
    selected_index: usize,
    /// Current input mode
    input_mode: InputMode,
    /// Search query when in search mode
    search_input: String,
    /// Port input when in editing mode
    port_input: String,
    /// Whether to sort projects by port (true) or name (false)
    sort_by_port: bool,
    /// Height of the visible viewport (set by UI)
    viewport_height: usize,
    /// Scroll offset for the list
    list_offset: usize,
    /// Set of collapsed folder names
    collapsed_folders: HashSet<String>,
}

/// Default implementation for NavigationHandler
/// 
/// The Default trait allows us to create a "default" instance.
/// This is commonly used for initialization in Rust.
impl Default for NavigationHandler {
    fn default() -> Self {
        Self {
            selected_index: 0,
            input_mode: InputMode::Normal,
            search_input: String::new(),
            port_input: String::new(),
            sort_by_port: false,
            viewport_height: 15, // Will be updated by UI
            list_offset: 0,
            collapsed_folders: HashSet::new(),
        }
    }
}

impl NavigationHandler {
    /// Creates a new NavigationHandler with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    // === Getters (Read-only access to internal state) ===
    
    /// Gets the currently selected index
    /// 
    /// The '&self' means we're borrowing the struct (not taking ownership).
    /// This is a "getter" method - it provides read-only access to internal data.
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }
    
    /// Gets the current input mode
    pub fn input_mode(&self) -> &InputMode {
        &self.input_mode  // Return a reference to the enum
    }
    
    /// Gets the current search input
    pub fn search_input(&self) -> &str {
        &self.search_input  // Return a string slice (borrowed string)
    }
    
    /// Gets the current port input
    pub fn port_input(&self) -> &str {
        &self.port_input
    }
    
    /// Gets the list offset for scrolling
    pub fn list_offset(&self) -> usize {
        self.list_offset
    }
    
    /// Checks if sorting by port
    pub fn is_sort_by_port(&self) -> bool {
        self.sort_by_port
    }
    
    // === Setters (Modify internal state) ===
    
    /// Sets the viewport height (called by UI when screen size changes)
    /// 
    /// The '&mut self' means we're borrowing the struct mutably.
    /// This allows us to modify the struct's internal state.
    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
    }
    
    // === Navigation Methods ===
    
    /// Moves cursor to next item (down)
    /// 
    /// Takes the current display items to know what's available.
    /// The Vec<DisplayItem> could be either full list or filtered list.
    pub fn move_next(&mut self, items: &[DisplayItem]) {
        if !items.is_empty() {
            // Use modulo arithmetic to wrap around to beginning
            self.selected_index = (self.selected_index + 1) % items.len();
            self.update_scroll_position(items);
        }
    }
    
    /// Moves cursor to previous item (up)
    pub fn move_previous(&mut self, items: &[DisplayItem]) {
        if !items.is_empty() {
            // Handle wraparound to end when at beginning
            self.selected_index = if self.selected_index == 0 {
                items.len() - 1  // Wrap to last item
            } else {
                self.selected_index - 1
            };
            self.update_scroll_position(items);
        }
    }
    
    /// Jumps to the first item
    pub fn move_to_top(&mut self, items: &[DisplayItem]) {
        if !items.is_empty() {
            self.selected_index = 0;
            self.update_scroll_position(items);
        }
    }
    
    /// Jumps to the last item
    pub fn move_to_bottom(&mut self, items: &[DisplayItem]) {
        if !items.is_empty() {
            self.selected_index = items.len() - 1;
            self.update_scroll_position(items);
        }
    }
    
    /// Pages up by half viewport height (like Vim's Ctrl+U)
    pub fn page_up(&mut self, items: &[DisplayItem]) {
        if !items.is_empty() {
            let half_page = self.viewport_height / 2;
            self.selected_index = self.selected_index.saturating_sub(half_page);
            self.update_scroll_position(items);
        }
    }
    
    /// Pages down by half viewport height (like Vim's Ctrl+D)
    pub fn page_down(&mut self, items: &[DisplayItem]) {
        if !items.is_empty() {
            let half_page = self.viewport_height / 2;
            self.selected_index = (self.selected_index + half_page).min(items.len() - 1);
            self.update_scroll_position(items);
        }
    }
    
    // === Input Mode Methods ===
    
    /// Starts editing mode for port input
    pub fn start_editing(&mut self, projects: &[ProjectInfo], items: &[DisplayItem]) -> Result<String, String> {
        // Check if we're on a valid project (not a header)
        if items.is_empty() || self.selected_index >= items.len() {
            return Err("No item selected".to_string());
        }
        
        let current_item = &items[self.selected_index];
        
        if current_item.is_header {
            return Err("Cannot set port on folder - select a project instead".to_string());
        }
        
        if let Some(project_index) = current_item.project_index {
            if project_index < projects.len() {
                self.input_mode = InputMode::Editing;
                self.port_input.clear();
                let project_name = &projects[project_index].name;
                return Ok(format!("Enter port for '{}' (ESC to cancel, ENTER to save)", project_name));
            }
        }
        
        Err("Invalid project selection".to_string())
    }
    
    /// Starts search mode
    pub fn start_search(&mut self) -> String {
        self.input_mode = InputMode::Searching;
        self.search_input.clear();
        "Search projects (ESC to cancel, ENTER to confirm): ".to_string()
    }
    
    /// Cancels current input mode and returns to normal
    pub fn cancel_input(&mut self) -> String {
        self.input_mode = InputMode::Normal;
        self.port_input.clear();
        self.search_input.clear();
        "Cancelled - hjkl/↑↓: navigate, r: set port, R: refresh, s: sort, /: search, e: expand, w: collapse".to_string()
    }

    /// Sets input mode to normal (used after confirming search)
    pub fn set_input_mode_normal(&mut self) {
        self.input_mode = InputMode::Normal;
    }
    
    
    // === Input Handling ===
    
    /// Adds a character to port input
    pub fn add_to_port_input(&mut self, c: char) {
        if self.input_mode == InputMode::Editing {
            self.port_input.push(c);
        }
    }
    
    /// Removes last character from port input
    pub fn backspace_port_input(&mut self) {
        if self.input_mode == InputMode::Editing {
            self.port_input.pop();
        }
    }
    
    /// Adds a character to search input
    pub fn add_to_search_input(&mut self, c: char) {
        if self.input_mode == InputMode::Searching {
            self.search_input.push(c);
        }
    }
    
    /// Removes last character from search input
    pub fn backspace_search_input(&mut self) {
        if self.input_mode == InputMode::Searching {
            self.search_input.pop();
        }
    }
    
    /// Gets the current port input as a parsed number
    pub fn get_port_number(&self) -> Result<u16, String> {
        self.port_input.parse::<u16>()
            .map_err(|_| "Invalid port number".to_string())
    }
    
    // === Sorting ===
    
    /// Toggles between sorting by name and sorting by port
    pub fn toggle_sort(&mut self) -> String {
        self.sort_by_port = !self.sort_by_port;
        let sort_mode = if self.sort_by_port { "port" } else { "name" };
        format!("Sorted by {} - hjkl/↑↓: navigate, r: set port, R: refresh, s: sort, /: search, e: expand, w: collapse", sort_mode)
    }
    
    // === Folder Collapse/Expand ===
    
    /// Toggles folder collapse state
    pub fn toggle_folder(&mut self, items: &[DisplayItem]) -> Option<String> {
        if self.selected_index >= items.len() {
            return None;
        }
        
        let current_item = &items[self.selected_index];
        
        if current_item.is_header {
            if let Some(folder_name) = self.extract_folder_name(&current_item.content) {
                if self.collapsed_folders.contains(&folder_name) {
                    self.collapsed_folders.remove(&folder_name);
                    Some(format!("Expanded folder '{}'", folder_name))
                } else {
                    self.collapsed_folders.insert(folder_name.clone());
                    Some(format!("Collapsed folder '{}'", folder_name))
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Expands a folder (removes from collapsed set)
    pub fn expand_folder(&mut self, items: &[DisplayItem], projects: &[ProjectInfo]) -> Option<String> {
        self.modify_folder_state(items, projects, false)
    }
    
    /// Collapses a folder (adds to collapsed set)
    pub fn collapse_folder(&mut self, items: &[DisplayItem], projects: &[ProjectInfo]) -> Option<String> {
        self.modify_folder_state(items, projects, true)
    }
    
    /// Gets the set of collapsed folders
    pub fn collapsed_folders(&self) -> &HashSet<String> {
        &self.collapsed_folders
    }
    
    /// Resets cursor position after display list changes
    pub fn reset_cursor_position(&mut self, items: &[DisplayItem]) {
        self.selected_index = 0;
        self.update_scroll_position(items);
    }
    
    
    // === Private Helper Methods ===
    
    /// Updates scroll position to keep selected item visible
    /// 
    /// This is private (no 'pub') because it's an internal implementation detail.
    fn update_scroll_position(&mut self, items: &[DisplayItem]) {
        if items.is_empty() {
            return;
        }
        
        // Keep cursor in the middle of the viewport when possible
        let half_viewport = self.viewport_height / 2;
        
        if self.selected_index >= half_viewport {
            self.list_offset = if self.selected_index + half_viewport >= items.len() {
                // Near the end, adjust offset to show as many items as possible
                items.len().saturating_sub(self.viewport_height)
            } else {
                // Keep selected item in the middle
                self.selected_index.saturating_sub(half_viewport)
            };
        } else {
            // Near the beginning, start from 0
            self.list_offset = 0;
        }
    }
    
    /// Modifies folder collapse state (expand or collapse)
    fn modify_folder_state(&mut self, items: &[DisplayItem], projects: &[ProjectInfo], should_collapse: bool) -> Option<String> {
        if self.selected_index >= items.len() {
            return None;
        }
        
        let current_item = &items[self.selected_index];
        let folder_name = if current_item.is_header {
            // If on header, use that folder
            self.extract_folder_name(&current_item.content)
        } else if let Some(project_index) = current_item.project_index {
            // If on project, use its parent folder
            if project_index < projects.len() {
                Some(projects[project_index].parent_folder.clone())
            } else {
                None
            }
        } else {
            None
        };
        
        if let Some(folder) = folder_name {
            if should_collapse {
                self.collapsed_folders.insert(folder.clone());
                Some(format!("Collapsed folder '{}' - hjkl/↑↓: navigate, e: expand, w: collapse", folder))
            } else {
                self.collapsed_folders.remove(&folder);
                Some(format!("Expanded folder '{}' - hjkl/↑↓: navigate, e: expand, w: collapse", folder))
            }
        } else {
            None
        }
    }
    
    /// Extracts folder name from header content string
    /// 
    /// Header content looks like: "📂 ▼ foldername/"
    /// We need to extract just "foldername"
    fn extract_folder_name(&self, content: &str) -> Option<String> {
        // Find the first space (after emoji)
        if let Some(start_pos) = content.find(' ') {
            let after_icon = &content[start_pos + 1..];
            // Find the second space (after expand/collapse indicator)
            if let Some(indicator_end) = after_icon.find(' ') {
                let after_indicator = &after_icon[indicator_end + 1..];
                // Find the trailing slash
                if let Some(slash_pos) = after_indicator.find('/') {
                    return Some(after_indicator[..slash_pos].to_string());
                }
            }
        }
        None
    }
}