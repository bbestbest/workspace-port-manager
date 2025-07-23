/*!
 * Main Application Entry Point
 *
 * This file demonstrates the SOLID principles in action:
 *
 * 1. **Single Responsibility Principle (SRP)**:
 *    - Each module has one clear responsibility
 *    - ProjectScanner: finds projects
 *    - NavigationHandler: manages cursor and input
 *    - UIRenderer: draws the interface
 *    - PortManager: handles .env file operations
 *    - DisplayBuilder: converts data for display
 *
 * 2. **Open/Closed Principle (OCP)**:
 *    - Easy to add new project types by extending ProjectScanner
 *    - Easy to add new UI themes by extending UIRenderer
 *    - Easy to add new key bindings in the event loop
 *
 * 3. **Liskov Substitution Principle (LSP)**:
 *    - All modules work through well-defined interfaces
 *    - Could swap implementations without breaking the app
 *
 * 4. **Interface Segregation Principle (ISP)**:
 *    - Each module only depends on what it needs
 *    - NavigationHandler doesn't know about file I/O
 *    - UIRenderer doesn't know about project scanning
 *
 * 5. **Dependency Inversion Principle (DIP)**:
 *    - High-level App struct depends on abstractions (modules)
 *    - Low-level details (file paths, colors) are isolated in modules
 *
 * Key Rust concepts for beginners:
 * - Module system: organizing code into separate files
 * - Error handling: Result<T, E> and ? operator
 * - Pattern matching: handling different input events
 * - Ownership: who owns what data and when
 * - Borrowing: accessing data without taking ownership
 */

// Import standard library items
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io};

// Import our custom modules
// The 'mod' declarations tell Rust about our module files
mod display_builder;
mod navigation;
mod port_manager;
mod project_scanner;
mod ui;

// Use statements bring specific items into scope
// This is like 'import' in other languages
use display_builder::DisplayBuilder;
use navigation::{DisplayItem, InputMode, NavigationHandler};
use port_manager::PortManager;
use project_scanner::{ProjectInfo, ProjectScanner};
use ui::UIRenderer;

/// Main Application State
///
/// This struct follows the Dependency Inversion Principle.
/// It depends on abstractions (our modules) rather than concrete implementations.
///
/// In Rust, structs are like classes but methods are defined separately.
struct App {
    /// All discovered projects
    projects: Vec<ProjectInfo>,

    /// Current display items (might be filtered)
    display_items: Vec<DisplayItem>,

    /// Filtered items when searching
    filtered_items: Vec<DisplayItem>,

    /// Current status message to show user
    status_message: String,

    /// Handles cursor navigation and input
    navigation: NavigationHandler,

    /// Scans directories for projects
    scanner: ProjectScanner,

    /// Renders the user interface
    ui_renderer: UIRenderer,

    /// Manages port assignments
    port_manager: PortManager,

    /// Builds display items from projects
    display_builder: DisplayBuilder,
}

/// Implementation block for App
///
/// This is where we define methods (functions) for our App struct.
/// In Rust, methods are defined separately from the struct definition.
impl App {
    /// Creates a new App instance
    ///
    /// This is our constructor. In Rust, constructors are just regular functions
    /// that return an instance of the struct. The 'Self' keyword refers to 'App'.
    fn new() -> Self {
        // Create all our modules
        let scanner = ProjectScanner::new();
        let navigation = NavigationHandler::new();
        let ui_renderer = UIRenderer::new();
        let port_manager = PortManager::new();
        let display_builder = DisplayBuilder::new();

        // Create the app with initial empty state
        let mut app = Self {
            projects: Vec::new(),
            display_items: Vec::new(),
            filtered_items: Vec::new(),
            status_message: "Ready - hjkl/↑↓: navigate, Ctrl+U/Ctrl+D: page up/down, r: set port, R: refresh, s: sort, /: search, e: expand, w: collapse, q: quit".to_string(),
            navigation,
            scanner,
            ui_renderer,
            port_manager,
            display_builder,
        };

        // Scan for projects on startup
        app.scan_projects();
        app
    }

    /// Scans for projects and updates display
    ///
    /// This method coordinates between the scanner and display builder.
    fn scan_projects(&mut self) {
        // Use the scanner to find all projects
        self.projects = self.scanner.scan_projects();

        // Update status message
        self.status_message = format!(
            "Found {} projects - hjkl/↑↓: navigate, Ctrl+U/Ctrl+D: page up/down, r: set port, R: refresh, s: sort, /: search, e: expand, w: collapse",
            self.projects.len()
        );

        // Build display items
        self.build_display_items();

        // Reset cursor to a valid position
        self.navigation.reset_cursor_position(&self.display_items);
    }

    /// Builds display items from current projects
    ///
    /// This method uses the display builder to convert project data
    /// into UI-ready display items.
    fn build_display_items(&mut self) {
        self.display_items = self.display_builder.build_display_items(
            &self.projects,
            self.navigation.collapsed_folders(),
            self.navigation.is_sort_by_port(),
        );

        // Update filtered items (used when searching)
        self.filtered_items = self.display_items.clone();
    }

    /// Gets the appropriate display items based on current mode
    ///
    /// This is a helper method that returns the right set of items
    /// depending on whether we're searching or not.
    fn get_current_items(&self) -> &[DisplayItem] {
        match self.navigation.input_mode() {
            InputMode::Searching => &self.filtered_items,
            _ => &self.display_items,
        }
    }

    /// Gets current items as a cloned vector to avoid borrowing issues
    fn get_current_items_cloned(&self) -> Vec<DisplayItem> {
        match self.navigation.input_mode() {
            InputMode::Searching => self.filtered_items.clone(),
            _ => self.display_items.clone(),
        }
    }

    /// Handles keyboard input events
    ///
    /// This is the main event handling method that processes all keyboard input.
    /// It uses pattern matching to handle different keys and input modes.
    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) {
        // Only process key press events (not releases)
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Handle input based on current mode
        // This is pattern matching - one of Rust's most powerful features
        match self.navigation.input_mode() {
            InputMode::Normal => self.handle_normal_mode_key(key),
            InputMode::Editing => self.handle_editing_mode_key(key),
            InputMode::Searching => self.handle_searching_mode_key(key),
        }
    }

    /// Handles keys in normal navigation mode
    fn handle_normal_mode_key(&mut self, key: crossterm::event::KeyEvent) {
        // Pattern match on the key code
        // Each arm handles a different key press
        match key.code {
            // Quit the application
            KeyCode::Char('q') => {
                // We'll handle this in the main loop
            }

            // Refresh projects
            KeyCode::Char('R') => {
                self.scan_projects();
            }

            // Start editing port
            KeyCode::Char('r') => {
                let items = self.get_current_items_cloned();
                match self.navigation.start_editing(&self.projects, &items) {
                    Ok(message) => self.status_message = message,
                    Err(error) => self.status_message = error,
                }
            }

            // Toggle sort mode
            KeyCode::Char('s') => {
                self.status_message = self.navigation.toggle_sort();
                self.build_display_items();
                self.navigation.reset_cursor_position(&self.display_items);
            }

            // Start search
            KeyCode::Char('/') => {
                self.status_message = self.navigation.start_search();
                self.filter_projects();
            }

            // Navigation keys
            KeyCode::Char('<') => {
                let items = self.get_current_items_cloned();
                self.navigation.move_to_top(&items);
            }
            KeyCode::Char('>') => {
                let items = self.get_current_items_cloned();
                self.navigation.move_to_bottom(&items);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let items = self.get_current_items_cloned();
                self.navigation.move_next(&items);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let items = self.get_current_items_cloned();
                self.navigation.move_previous(&items);
            }

            // Page navigation (Ctrl+U and Ctrl+D like vim)
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let items = self.get_current_items_cloned();
                self.navigation.page_up(&items);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let items = self.get_current_items_cloned();
                self.navigation.page_down(&items);
            }

            // Folder operations
            KeyCode::Char('e') => {
                let items = self.get_current_items_cloned();
                if let Some(message) = self.navigation.expand_folder(&items, &self.projects) {
                    self.status_message = message;
                    self.rebuild_after_folder_change();
                }
            }
            KeyCode::Char('w') => {
                let items = self.get_current_items_cloned();
                if let Some(message) = self.navigation.collapse_folder(&items, &self.projects) {
                    self.status_message = message;
                    self.rebuild_after_folder_change();
                }
            }
            KeyCode::Enter => {
                let items = self.get_current_items_cloned();
                if let Some(message) = self.navigation.toggle_folder(&items) {
                    self.status_message = message;
                    self.rebuild_after_folder_change();
                }
            }

            // Vim navigation (h/l reserved for future use)
            KeyCode::Char('h') | KeyCode::Char('l') => {
                // No action for now
            }

            // Ignore other keys
            _ => {}
        }
    }

    /// Handles keys in port editing mode
    fn handle_editing_mode_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            // Save the port
            KeyCode::Enter => {
                self.save_port();
            }

            // Cancel editing
            KeyCode::Esc => {
                self.status_message = self.navigation.cancel_input();
            }

            // Add character to input
            KeyCode::Char(c) => {
                self.navigation.add_to_port_input(c);
            }

            // Remove last character
            KeyCode::Backspace => {
                self.navigation.backspace_port_input();
            }

            // Ignore other keys
            _ => {}
        }
    }

    /// Handles keys in search mode
    fn handle_searching_mode_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            // Confirm search
            KeyCode::Enter => {
                let result_count = self
                    .filtered_items
                    .iter()
                    .filter(|item| !item.is_header)
                    .count();
                self.status_message = self.navigation.confirm_search(result_count);
            }

            // Cancel search
            KeyCode::Esc => {
                self.status_message = self.navigation.cancel_input();
                self.filtered_items = self.display_items.clone();
                self.navigation.reset_cursor_position(&self.display_items);
            }

            // Add character to search
            KeyCode::Char(c) => {
                self.navigation.add_to_search_input(c);
                self.filter_projects();
            }

            // Remove character from search
            KeyCode::Backspace => {
                self.navigation.backspace_search_input();
                self.filter_projects();
            }

            // Ignore other keys
            _ => {}
        }
    }

    /// Saves the currently entered port
    fn save_port(&mut self) {
        // Get the port number from navigation
        match self.navigation.get_port_number() {
            Ok(port) => {
                // Get current project
                let items = &self.filtered_items;
                let selected_index = self.navigation.selected_index();

                if let Some(project_index) = items
                    .get(selected_index)
                    .and_then(|item| item.project_index)
                {
                    if let Some(project) = self.projects.get(project_index).cloned() {
                        // Use port manager to save the port
                        match self.port_manager.set_port(&project, port) {
                            Ok(()) => {
                                // Update the project in our list
                                self.projects[project_index].port = Some(port);
                                self.build_display_items();
                                self.status_message =
                                    format!("✓ Updated port for '{}' to {}", project.name, port);
                            }
                            Err(e) => {
                                self.status_message = format!("✗ Error: {}", e);
                            }
                        }
                    }
                }
            }
            Err(error) => {
                self.status_message = error;
            }
        }

        // Always return to normal mode after save attempt
        self.navigation.cancel_input();
    }

    /// Filters projects based on search input
    fn filter_projects(&mut self) {
        self.filtered_items = self
            .display_builder
            .filter_display_items(&self.display_items, self.navigation.search_input());

        // Reset cursor to first valid item
        self.navigation.reset_cursor_position(&self.filtered_items);
    }

    /// Rebuilds display after folder expand/collapse operations
    fn rebuild_after_folder_change(&mut self) {
        self.build_display_items();

        // If we're searching, re-apply the filter
        if *self.navigation.input_mode() == InputMode::Searching {
            self.filter_projects();
        } else {
            self.filtered_items = self.display_items.clone();
        }
    }
}

/// Main event loop
///
/// This function runs the application's main loop, handling events and rendering the UI.
/// It demonstrates Rust's error handling with Result<T, E>.
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    mut app: App,
) -> io::Result<()> {
    // Main application loop
    loop {
        // Render the UI
        // The |f| syntax is a closure (anonymous function)
        terminal.draw(|f| {
            let items = match app.navigation.input_mode() {
                InputMode::Searching => &app.filtered_items,
                _ => &app.display_items,
            };
            app.ui_renderer.render(
                f,
                &mut app.navigation,
                &app.projects,
                items,
                &app.status_message,
            );
        })?;

        // Handle events
        // The ? operator returns early if there's an error
        if let Event::Key(key) = event::read()? {
            // Check for quit key in normal mode
            if *app.navigation.input_mode() == InputMode::Normal
                && key.code == KeyCode::Char('q')
                && key.kind == KeyEventKind::Press
            {
                return Ok(()); // Exit the application
            }

            // Handle the key event
            app.handle_key_event(key);
        }
    }
}

/// Main function - application entry point
///
/// This function sets up the terminal, runs the app, and cleans up.
/// The Result<(), Box<dyn Error>> return type means this function
/// can return any kind of error.
fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal for TUI mode
    enable_raw_mode()?; // Enable raw input mode
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?; // Enter full-screen mode
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run application
    let app = App::new();
    let result = run_app(&mut terminal, app);

    // Restore terminal to normal mode
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Print any errors that occurred
    if let Err(err) = result {
        println!("{:?}", err);
    }

    Ok(())
}

/*
 * SOLID Principles Demonstrated:
 *
 * 1. **Single Responsibility Principle (SRP)**:
 *    - ProjectScanner: Only scans for projects
 *    - NavigationHandler: Only handles cursor/input
 *    - UIRenderer: Only renders the interface
 *    - PortManager: Only manages .env files
 *    - DisplayBuilder: Only builds display items
 *
 * 2. **Open/Closed Principle (OCP)**:
 *    - Easy to add new project types in ProjectScanner
 *    - Easy to add new key bindings in event handlers
 *    - Easy to change UI styling in UIRenderer
 *
 * 3. **Liskov Substitution Principle (LSP)**:
 *    - All modules have clear interfaces
 *    - Could swap implementations without breaking App
 *
 * 4. **Interface Segregation Principle (ISP)**:
 *    - Each module only exposes what others need
 *    - No module depends on unused functionality
 *
 * 5. **Dependency Inversion Principle (DIP)**:
 *    - App depends on module abstractions, not implementations
 *    - Easy to test by mocking individual modules
 *
 * Key Rust Concepts for Beginners:
 *
 * 1. **Module System**:
 *    - `mod module_name;` declares a module
 *    - `use module::Item;` brings items into scope
 *    - Modules provide namespace separation
 *
 * 2. **Error Handling**:
 *    - Result<T, E> for operations that can fail
 *    - ? operator for early return on errors
 *    - No exceptions - all errors are values
 *
 * 3. **Pattern Matching**:
 *    - `match` expressions handle all cases
 *    - Compiler ensures you handle every possibility
 *    - Very powerful for handling enums and complex data
 *
 * 4. **Ownership and Borrowing**:
 *    - `&self` borrows read-only
 *    - `&mut self` borrows mutably
 *    - Rust prevents data races at compile time
 *
 * 5. **Memory Safety**:
 *    - No null pointers (use Option<T> instead)
 *    - No buffer overflows (bounds checking)
 *    - No memory leaks (automatic cleanup)
 */

