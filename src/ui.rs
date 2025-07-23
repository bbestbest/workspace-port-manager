/*!
 * UI Rendering Module
 * 
 * This module follows the Single Responsibility Principle (SRP).
 * It has ONE job: rendering the user interface using ratatui.
 * 
 * Key Rust concepts demonstrated:
 * - Traits: Shared behavior between types (like interfaces)
 * - Lifetimes: How long references are valid (the 'a in &'a str)
 * - Pattern matching: Destructuring data and handling different cases
 * - Iterators: Functional programming style data processing
 * - Closures: Anonymous functions (like lambdas)
 */

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::navigation::{DisplayItem, InputMode, NavigationHandler};
use crate::project_scanner::ProjectInfo;

/// UI Renderer - responsible for drawing the interface
/// 
/// This struct follows SRP: it only handles UI rendering.
/// It doesn't know about navigation logic or project scanning.
pub struct UIRenderer;

impl UIRenderer {
    /// Creates a new UI renderer
    pub fn new() -> Self {
        Self
    }
    
    /// Renders the complete UI
    /// 
    /// This is the main rendering method that coordinates all UI components.
    /// 
    /// Parameters:
    /// - f: The ratatui Frame to draw on
    /// - navigation: Current navigation state
    /// - projects: List of all projects
    /// - display_items: Current display items (might be filtered)
    /// - status_message: Current status message to show
    /// 
    /// The 'f parameter has a lifetime 'a - this means the Frame reference
    /// must live at least as long as this function call.
    pub fn render<'a>(
        &self,
        f: &mut Frame<'a>,
        navigation: &mut NavigationHandler,
        _projects: &[ProjectInfo],
        display_items: &[DisplayItem],
        status_message: &str,
    ) {
        // Create the main layout with three sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)  // Stack vertically
            .margin(1)                       // 1-character margin around the edge
            .constraints([
                Constraint::Length(3),       // Header: fixed 3 lines
                Constraint::Min(0),          // Main content: take remaining space
                Constraint::Length(3),       // Status bar: fixed 3 lines
            ])
            .split(f.area());               // Split the entire terminal area
        
        // Render each section
        self.render_header(f, chunks[0]);
        self.render_main_content(f, chunks[1], navigation, display_items);
        self.render_status_bar(f, chunks[2], status_message);
    }
    
    /// Renders the header section
    /// 
    /// The header shows the application title and is always visible.
    fn render_header(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let header = Paragraph::new("🔌 Port Management - ~/Workspace Projects")
            .style(
                Style::default()
                    .fg(Color::Cyan)              // Cyan text color
                    .add_modifier(Modifier::BOLD) // Make text bold
            )
            .alignment(Alignment::Center)         // Center the text
            .block(Block::default().borders(Borders::ALL)); // Add border around it
        
        f.render_widget(header, area);
    }
    
    /// Renders the main content area (project list + input panel)
    /// 
    /// This splits the main area horizontally into two sections.
    fn render_main_content(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        navigation: &mut NavigationHandler,
        display_items: &[DisplayItem],
    ) {
        // Split main area horizontally
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70),    // Project list: 70% of width
                Constraint::Percentage(30),    // Input panel: 30% of width
            ])
            .split(area);
        
        // Update navigation's viewport height based on actual area size
        // Subtract 2 for the borders (top and bottom)
        let viewport_height = (main_chunks[0].height.saturating_sub(2)) as usize;
        navigation.set_viewport_height(viewport_height);
        
        // Render both sections
        self.render_project_list(f, main_chunks[0], navigation, display_items);
        self.render_input_panel(f, main_chunks[1], navigation);
    }
    
    /// Renders the project list
    /// 
    /// This is the main interactive area where users see and navigate projects.
    fn render_project_list(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        navigation: &NavigationHandler,
        display_items: &[DisplayItem],
    ) {
        let selected_index = navigation.selected_index();
        let list_offset = navigation.list_offset();
        let viewport_height = (area.height.saturating_sub(2)) as usize;
        
        // Create list items for the visible portion of the list
        // This is an example of iterator chaining - a common Rust pattern
        let projects: Vec<ListItem> = display_items
            .iter()                          // Create iterator over items
            .enumerate()                     // Add index to each item (index, item)
            .skip(list_offset)              // Skip items before visible area
            .take(viewport_height)          // Take only visible items
            .map(|(i, item)| {              // Transform each (index, item) pair
                self.create_list_item(i, item, selected_index)
            })
            .collect();                     // Collect into Vec<ListItem>
        
        // Create the list widget
        let projects_list = List::new(projects)
            .block(Block::default().title("Projects").borders(Borders::ALL));
        
        f.render_widget(projects_list, area);
    }
    
    /// Creates a single list item with appropriate styling
    /// 
    /// This helper method encapsulates the logic for styling individual list items.
    fn create_list_item(&self, index: usize, item: &DisplayItem, selected_index: usize) -> ListItem {
        let is_selected = index == selected_index;
        let content = vec![Line::from(item.content.clone())];
        
        // Apply different styles based on item type and selection state
        let style = if is_selected {
            // Selected item: dark gray background with bold text
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else if item.is_header {
            // Folder header: cyan text with bold
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            // Regular project item: default styling
            Style::default()
        };
        
        ListItem::new(content).style(style)
    }
    
    /// Renders the input panel (controls/help text)
    /// 
    /// Shows different content based on current input mode.
    fn render_input_panel(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        navigation: &NavigationHandler,
    ) {
        // Choose content based on current input mode
        // This uses pattern matching - one of Rust's most powerful features
        let input_text = match navigation.input_mode() {
            InputMode::Normal => self.get_normal_mode_text(),
            InputMode::Editing => self.get_editing_mode_text(navigation),
            InputMode::Searching => self.get_searching_mode_text(navigation),
        };
        
        let input = Paragraph::new(input_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().title("Input").borders(Borders::ALL));
        
        f.render_widget(input, area);
    }
    
    /// Gets help text for normal navigation mode
    fn get_normal_mode_text(&self) -> Vec<Line<'static>> {
        vec![
            Line::from("Controls:"),
            Line::from(""),
            Line::from("k/↑  Up"),
            Line::from("j/↓  Down"),
            Line::from("^U   Page up"),
            Line::from("^D   Page down"),
            Line::from("r    Set port"),
            Line::from("R    Refresh"),
            Line::from("s    Sort"),
            Line::from("/    Search"),
            Line::from("</>  Top/Bot"),
            Line::from("e    Expand"),
            Line::from("w    Collapse"),
            Line::from("ENT  Toggle"),
            Line::from("q    Quit"),
        ]
    }
    
    /// Gets input display for port editing mode
    fn get_editing_mode_text<'a>(&self, navigation: &'a NavigationHandler) -> Vec<Line<'a>> {
        vec![
            Line::from("Enter port:"),
            Line::from(""),
            Line::from(navigation.port_input()),  // Show current input
            Line::from(""),
            Line::from("ENTER  Save"),
            Line::from("ESC    Cancel"),
        ]
    }
    
    /// Gets input display for search mode
    fn get_searching_mode_text<'a>(&self, navigation: &'a NavigationHandler) -> Vec<Line<'a>> {
        vec![
            Line::from("Search:"),
            Line::from(""),
            Line::from(navigation.search_input()),  // Show current search query
            Line::from(""),
            Line::from("ENTER  Confirm"),
            Line::from("ESC    Cancel"),
        ]
    }
    
    /// Renders the status bar
    /// 
    /// Shows current status messages and feedback to the user.
    fn render_status_bar(&self, f: &mut Frame, area: ratatui::layout::Rect, status_message: &str) {
        let status = Paragraph::new(status_message)
            .style(Style::default().fg(Color::Yellow))  // Yellow text for visibility
            .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(status, area);
    }
}

/// Default implementation for UIRenderer
/// 
/// This allows UIRenderer to be created with UIRenderer::default()
/// It's a common Rust pattern for types that don't need special initialization.
impl Default for UIRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/*
 * Key Rust Concepts Explained:
 * 
 * 1. **Ownership and Borrowing**:
 *    - `&self` borrows the struct (read-only access)
 *    - `&mut navigation` borrows mutably (can modify)
 *    - `&[DisplayItem]` borrows a slice of the vector (read-only)
 * 
 * 2. **Lifetimes**:
 *    - `<'a>` in function signatures indicates how long references live
 *    - Ensures memory safety without garbage collection
 * 
 * 3. **Pattern Matching**:
 *    - `match` expressions handle all possible cases
 *    - Compiler ensures you handle every variant of an enum
 * 
 * 4. **Iterator Chaining**:
 *    - `.iter().enumerate().skip().take().map().collect()`
 *    - Functional programming style that's memory efficient
 *    - Lazy evaluation - only processes what's needed
 * 
 * 5. **Traits**:
 *    - Default trait provides standard initialization
 *    - Traits are like interfaces but more powerful
 * 
 * 6. **Error Handling**:
 *    - Result<T, E> for operations that can fail
 *    - Option<T> for values that might not exist
 *    - No null pointer exceptions!
 */