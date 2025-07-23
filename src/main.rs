use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, HashSet}, error::Error, fs, io, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectInfo {
    name: String,
    port: Option<u16>,
    path: String,
    parent_folder: String,
}

#[derive(Debug, Clone)]
struct DisplayItem {
    content: String,
    is_header: bool,
    project_index: Option<usize>,
}

struct App {
    projects: Vec<ProjectInfo>,
    display_items: Vec<DisplayItem>,
    filtered_items: Vec<DisplayItem>,
    selected_index: usize,
    input_mode: InputMode,
    port_input: String,
    search_input: String,
    status_message: String,
    sort_by_port: bool,
    viewport_height: usize,
    list_offset: usize,
    collapsed_folders: HashSet<String>,
}

#[derive(Debug, PartialEq)]
enum InputMode {
    Normal,
    Editing,
    Searching,
}

impl App {
    fn new() -> App {
        let mut app = App {
            projects: Vec::new(),
            display_items: Vec::new(),
            filtered_items: Vec::new(),
            selected_index: 0,
            input_mode: InputMode::Normal,
            port_input: String::new(),
            search_input: String::new(),
            status_message: "Ready - hjkl/↑↓: navigate, r: set port, R: refresh, s: sort, /: search, e: expand, w: collapse, q: quit"
                .to_string(),
            sort_by_port: false,
            viewport_height: 15, // Default value, will be updated by UI
            list_offset: 0,
            collapsed_folders: HashSet::new(),
        };
        app.scan_projects();
        app.update_list_state();
        app
    }

    fn scan_projects(&mut self) {
        let workspace_path = dirs::home_dir()
            .map(|home| home.join("Workspace"))
            .unwrap_or_else(|| Path::new("~/Workspace").to_path_buf());

        self.projects.clear();

        if workspace_path.exists() {
            scan_recursive(&workspace_path, &mut self.projects, &workspace_path);
        }

        self.status_message = format!(
            "Found {} projects - hjkl/↑↓: navigate, r: set port, R: refresh, s: sort, /: search, e: expand, w: collapse",
            self.projects.len()
        );
        self.build_display_items();

        // Find first non-header item and select it
        self.selected_index = 0;
        for (i, item) in self.display_items.iter().enumerate() {
            if !item.is_header {
                self.selected_index = i;
                break;
            }
        }
        self.update_list_state();
    }

    fn build_display_items(&mut self) {
        self.display_items.clear();

        let mut grouped_projects: BTreeMap<String, Vec<(usize, &ProjectInfo)>> = BTreeMap::new();

        for (index, project) in self.projects.iter().enumerate() {
            grouped_projects
                .entry(project.parent_folder.clone())
                .or_insert_with(Vec::new)
                .push((index, project));
        }

        for (folder, mut projects) in grouped_projects {
            let is_collapsed = self.collapsed_folders.contains(&folder);
            let expand_indicator = if is_collapsed { "▶" } else { "▼" };
            
            // Add folder header
            self.display_items.push(DisplayItem {
                content: format!("📂 {} {}/", expand_indicator, folder),
                is_header: true,
                project_index: None,
            });

            // Sort projects within folder
            if self.sort_by_port {
                projects.sort_by(|(_, a), (_, b)| match (a.port, b.port) {
                    (Some(port_a), Some(port_b)) => port_a.cmp(&port_b),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.name.cmp(&b.name),
                });
            } else {
                projects.sort_by(|(_, a), (_, b)| a.name.cmp(&b.name));
            }

            // Add projects in folder (only if not collapsed)
            if !is_collapsed {
                for (original_index, project) in projects {
                let port_text = match project.port {
                    Some(port) => format!(":{}", port),
                    None => "─".to_string(),
                };

                let project_name = project
                    .name
                    .strip_prefix(&format!("{}/", folder))
                    .unwrap_or(&project.name);

                // Truncate project name to fit within column width
                let max_name_width = 35;
                let truncated_name = if project_name.len() > max_name_width {
                    format!("{}…", &project_name[..max_name_width.saturating_sub(1)])
                } else {
                    project_name.to_string()
                };

                    self.display_items.push(DisplayItem {
                        content: format!("  📁 {:<35} {}", truncated_name, port_text),
                        is_header: false,
                        project_index: Some(original_index),
                    });
                }
            }
        }
        
        self.filtered_items = self.display_items.clone();
    }

    fn update_list_state(&mut self) {
        let items = if self.input_mode == InputMode::Searching {
            &self.filtered_items
        } else {
            &self.display_items
        };
        
        if !items.is_empty() {
            // Keep cursor in the middle of the viewport
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
    }

    fn next_project(&mut self) {
        let items = if self.input_mode == InputMode::Searching {
            &self.filtered_items
        } else {
            &self.display_items
        };
        
        if !items.is_empty() {
            self.selected_index = (self.selected_index + 1) % items.len();
            self.update_list_state();
        }
    }

    fn previous_project(&mut self) {
        let items = if self.input_mode == InputMode::Searching {
            &self.filtered_items
        } else {
            &self.display_items
        };
        
        if !items.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                items.len() - 1
            } else {
                self.selected_index - 1
            };
            self.update_list_state();
        }
    }

    fn start_editing(&mut self) {
        let items = if self.input_mode == InputMode::Searching {
            &self.filtered_items
        } else {
            &self.display_items
        };
        
        if !items.is_empty() && !items[self.selected_index].is_header {
            if let Some(project_index) = items[self.selected_index].project_index {
                self.input_mode = InputMode::Editing;
                self.port_input.clear();
                let project_name = &self.projects[project_index].name;
                self.status_message = format!(
                    "Enter port for '{}' (ESC to cancel, ENTER to save)",
                    project_name
                );
            }
        } else if !items.is_empty() && items[self.selected_index].is_header {
            self.status_message = "Cannot set port on folder - select a project instead".to_string();
        }
    }

    fn save_port(&mut self) {
        if let Ok(port) = self.port_input.parse::<u16>() {
                let items = &self.filtered_items;
            
            if let Some(project_index) = items[self.selected_index].project_index {
                let project_name = self.projects[project_index].name.clone();
                let workspace_path = dirs::home_dir()
                    .map(|home| home.join("Workspace"))
                    .unwrap_or_else(|| Path::new("~/Workspace").to_path_buf());

                let project_path = workspace_path.join(&project_name);
                let env_file = project_path.join(".env");

                let env_content = if env_file.exists() {
                    fs::read_to_string(&env_file).unwrap_or_default()
                } else {
                    String::new()
                };

                let mut port_found = false;
                let mut new_content = String::new();

                for line in env_content.lines() {
                    if line.starts_with("PORT=") {
                        new_content.push_str(&format!("PORT={}\n", port));
                        port_found = true;
                    } else {
                        new_content.push_str(line);
                        new_content.push('\n');
                    }
                }

                if !port_found {
                    new_content.push_str(&format!("PORT={}\n", port));
                }

                match fs::write(&env_file, new_content) {
                    Ok(_) => {
                        self.projects[project_index].port = Some(port);
                        self.build_display_items();
                        self.filtered_items = self.display_items.clone();
                        self.status_message =
                            format!("✓ Updated port for '{}' to {}", project_name, port);
                    }
                    Err(e) => {
                        self.status_message = format!("✗ Error: {}", e);
                    }
                }
            } else {
                self.status_message = "No project selected".to_string();
            }
        } else {
            self.status_message = "Invalid port number".to_string();
        }

        self.input_mode = InputMode::Normal;
        self.port_input.clear();
    }

    fn cancel_editing(&mut self) {
        self.input_mode = InputMode::Normal;
        self.port_input.clear();
        self.status_message =
            "Cancelled - hjkl/↑↓: navigate, r: set port, R: refresh, s: sort, /: search, e: expand, w: collapse".to_string();
    }

    fn toggle_sort(&mut self) {
        self.sort_by_port = !self.sort_by_port;
        let sort_mode = if self.sort_by_port { "port" } else { "name" };
        self.status_message = format!(
            "Sorted by {} - hjkl/↑↓: navigate, r: set port, R: refresh, s: sort, /: search, e: expand, w: collapse",
            sort_mode
        );
        self.build_display_items();

        // Find first non-header item and select it
        self.selected_index = 0;
        for (i, item) in self.display_items.iter().enumerate() {
            if !item.is_header {
                self.selected_index = i;
                break;
            }
        }
        self.update_list_state();
    }
    
    fn go_to_top(&mut self) {
        let items = if self.input_mode == InputMode::Searching {
            &self.filtered_items
        } else {
            &self.display_items
        };
        
        if !items.is_empty() {
            self.selected_index = 0;
            self.update_list_state();
        }
    }
    
    fn go_to_bottom(&mut self) {
        let items = if self.input_mode == InputMode::Searching {
            &self.filtered_items
        } else {
            &self.display_items
        };
        
        if !items.is_empty() {
            self.selected_index = items.len() - 1;
            self.update_list_state();
        }
    }
    
    fn page_up(&mut self) {
        let items = if self.input_mode == InputMode::Searching {
            &self.filtered_items
        } else {
            &self.display_items
        };
        
        if !items.is_empty() {
            let half_page = self.viewport_height / 2;
            self.selected_index = self.selected_index.saturating_sub(half_page);
            self.update_list_state();
        }
    }
    
    fn page_down(&mut self) {
        let items = if self.input_mode == InputMode::Searching {
            &self.filtered_items
        } else {
            &self.display_items
        };
        
        if !items.is_empty() {
            let half_page = self.viewport_height / 2;
            self.selected_index = (self.selected_index + half_page).min(items.len() - 1);
            self.update_list_state();
        }
    }
    
    fn expand_folder(&mut self) {
        self.toggle_folder_state(false); // false means expand (remove from collapsed set)
    }
    
    fn collapse_folder(&mut self) {
        self.toggle_folder_state(true); // true means collapse (add to collapsed set)
    }
    
    fn toggle_folder_state(&mut self, should_collapse: bool) {
        let items = if self.input_mode == InputMode::Searching {
            &self.filtered_items
        } else {
            &self.display_items
        };
        
        if self.selected_index < items.len() {
            let current_item = &items[self.selected_index];
            let folder_name = if current_item.is_header {
                // If on header, use that folder
                self.extract_folder_name(&current_item.content)
            } else if let Some(project_index) = current_item.project_index {
                // If on project, use its parent folder
                Some(self.projects[project_index].parent_folder.clone())
            } else {
                None
            };
            
            if let Some(folder) = folder_name {
                if should_collapse {
                    self.collapsed_folders.insert(folder.clone());
                    self.status_message = format!("Collapsed folder '{}' - hjkl/↑↓: navigate, e: expand, w: collapse", folder);
                } else {
                    self.collapsed_folders.remove(&folder);
                    self.status_message = format!("Expanded folder '{}' - hjkl/↑↓: navigate, e: expand, w: collapse", folder);
                }
                
                self.build_display_items();
                if self.input_mode == InputMode::Searching {
                    self.filter_projects();
                } else {
                    self.filtered_items = self.display_items.clone();
                }
                
                // After collapse/expand, try to position cursor on the folder header
                let items = if self.input_mode == InputMode::Searching {
                    &self.filtered_items
                } else {
                    &self.display_items
                };
                
                // Find the folder header we just operated on
                for (i, item) in items.iter().enumerate() {
                    if item.is_header && item.content.contains(&folder) {
                        self.selected_index = i;
                        break;
                    }
                }
                
                self.update_list_state();
            }
        }
    }
    
    fn toggle_folder(&mut self) {
        let items = if self.input_mode == InputMode::Searching {
            &self.filtered_items
        } else {
            &self.display_items
        };
        
        if self.selected_index < items.len() {
            let current_item = &items[self.selected_index];
            
            if current_item.is_header {
                // Extract folder name from header content
                if let Some(folder_name) = self.extract_folder_name(&current_item.content) {
                    if self.collapsed_folders.contains(&folder_name) {
                        self.collapsed_folders.remove(&folder_name);
                    } else {
                        self.collapsed_folders.insert(folder_name);
                    }
                    
                    self.build_display_items();
                    if self.input_mode == InputMode::Searching {
                        self.filter_projects();
                    } else {
                        self.filtered_items = self.display_items.clone();
                    }
                    self.update_list_state();
                }
            }
        }
    }
    
    fn extract_folder_name(&self, content: &str) -> Option<String> {
        // Extract folder name from content like "📂 ▼ folder/"
        if let Some(start_pos) = content.find(' ') {
            let after_icon = &content[start_pos + 1..];
            if let Some(indicator_end) = after_icon.find(' ') {
                let after_indicator = &after_icon[indicator_end + 1..];
                if let Some(slash_pos) = after_indicator.find('/') {
                    return Some(after_indicator[..slash_pos].to_string());
                }
            }
        }
        None
    }
    
    fn start_search(&mut self) {
        self.input_mode = InputMode::Searching;
        self.search_input.clear();
        self.status_message = "Search projects (ESC to cancel, ENTER to confirm): ".to_string();
        self.filter_projects();
    }
    
    fn filter_projects(&mut self) {
        if self.search_input.is_empty() {
            self.filtered_items = self.display_items.clone();
        } else {
            let search_lower = self.search_input.to_lowercase();
            self.filtered_items = self.display_items
                .iter()
                .filter(|item| {
                    if item.is_header {
                        // Include headers if any project in that folder matches
                        let folder_name = item.content.strip_prefix("📂 ")
                            .unwrap_or(&item.content)
                            .strip_suffix("/")
                            .unwrap_or(&item.content)
                            .to_lowercase();
                        folder_name.contains(&search_lower)
                    } else {
                        // Search in project name and port
                        let content_lower = item.content.to_lowercase();
                        content_lower.contains(&search_lower)
                    }
                })
                .cloned()
                .collect();
        }
        
        // Reset selection to first non-header item
        self.selected_index = 0;
        for (i, item) in self.filtered_items.iter().enumerate() {
            if !item.is_header {
                self.selected_index = i;
                break;
            }
        }
        
        self.update_list_state();
    }
    
    fn cancel_search(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_input.clear();
        self.filtered_items = self.display_items.clone();
        self.status_message = "Search cancelled - hjkl/↑↓: navigate, r: set port, R: refresh, s: sort, /: search, e: expand, w: collapse".to_string();
        
        // Reset selection to first non-header item
        self.selected_index = 0;
        for (i, item) in self.display_items.iter().enumerate() {
            if !item.is_header {
                self.selected_index = i;
                break;
            }
        }
        
        self.update_list_state();
    }
    
    fn confirm_search(&mut self) {
        self.input_mode = InputMode::Normal;
        let result_count = self.filtered_items.iter().filter(|item| !item.is_header).count();
        self.status_message = format!("Found {} matches - hjkl/↑↓: navigate, r: set port, R: refresh, s: sort, /: search, e: expand, w: collapse", result_count);
        self.update_list_state();
    }
}

fn scan_recursive(dir_path: &Path, projects: &mut Vec<ProjectInfo>, workspace_root: &Path) {
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if dir_name.starts_with('.')
                            || dir_name == "node_modules"
                            || dir_name == "target"
                            || dir_name == "build"
                            || dir_name == "dist"
                            || dir_name == "__pycache__"
                        {
                            continue;
                        }
                    }

                    if is_working_project(&path) {
                        let relative_path = path
                            .strip_prefix(workspace_root)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .to_string();

                        let parent_folder = relative_path
                            .split('/')
                            .next()
                            .unwrap_or("other")
                            .to_string();

                        let port = detect_project_port(&path);
                        projects.push(ProjectInfo {
                            name: relative_path.clone(),
                            port,
                            path: path.to_string_lossy().to_string(),
                            parent_folder,
                        });
                    } else {
                        let depth = path
                            .strip_prefix(workspace_root)
                            .map(|p| p.components().count())
                            .unwrap_or(0);

                        if depth < 3 {
                            scan_recursive(&path, projects, workspace_root);
                        }
                    }
                }
            }
        }
    }
}

fn is_working_project(project_path: &Path) -> bool {
    let project_files = [
        "package.json",
        "Cargo.toml",
        "pom.xml",
        "build.gradle",
        "requirements.txt",
        "setup.py",
        "pyproject.toml",
        "go.mod",
        "composer.json",
        "Gemfile",
        "mix.exs",
        "pubspec.yaml",
        "CMakeLists.txt",
        "Makefile",
        "docker-compose.yml",
        "docker-compose.yaml",
        "Dockerfile",
        ".gitignore",
    ];

    if let Ok(entries) = fs::read_dir(project_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let file_name = entry.file_name();
                if let Some(name) = file_name.to_str() {
                    if name.ends_with(".sln") || project_files.contains(&name) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn detect_project_port(project_path: &Path) -> Option<u16> {
    let env_file = project_path.join(".env");
    if env_file.exists() {
        if let Ok(content) = fs::read_to_string(&env_file) {
            for line in content.lines() {
                if line.starts_with("PORT=") {
                    if let Ok(port) = line.trim_start_matches("PORT=").parse::<u16>() {
                        return Some(port);
                    }
                }
            }
        }
    }
    None
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(f.area());

    // Header
    let header = Paragraph::new("🔌 Port Management - ~/Workspace Projects")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Main content area
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70), // Project list
            Constraint::Percentage(30), // Input area
        ])
        .split(chunks[1]);

    // Update viewport height based on the actual project list area
    app.viewport_height = (main_chunks[0].height.saturating_sub(2)) as usize; // Subtract 2 for borders

    // Project list
    let items = if app.input_mode == InputMode::Searching {
        &app.filtered_items
    } else {
        &app.display_items
    };
    
    let projects: Vec<ListItem> = items
        .iter()
        .enumerate()
        .skip(app.list_offset)
        .take(app.viewport_height)
        .map(|(i, item)| {
            // Adjust the index for highlighting since we're using skip/take
            let is_selected = i == app.selected_index;
            let content = vec![Line::from(item.content.clone())];

            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else if item.is_header {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let projects_list = List::new(projects)
        .block(Block::default().title("Projects").borders(Borders::ALL));

    f.render_widget(projects_list, main_chunks[0]);

    // Input area
    let input_text = match app.input_mode {
        InputMode::Normal => {
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
        InputMode::Editing => {
            vec![
                Line::from("Enter port:"),
                Line::from(""),
                Line::from(app.port_input.as_str()),
                Line::from(""),
                Line::from("ENTER  Save"),
                Line::from("ESC    Cancel"),
            ]
        }
        InputMode::Searching => {
            vec![
                Line::from("Search:"),
                Line::from(""),
                Line::from(app.search_input.as_str()),
                Line::from(""),
                Line::from("ENTER  Confirm"),
                Line::from("ESC    Cancel"),
            ]
        }
    };

    let input = Paragraph::new(input_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().title("Input").borders(Borders::ALL));
    f.render_widget(input, main_chunks[1]);

    // Status bar
    let status = Paragraph::new(app.status_message.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    mut app: App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('R') => app.scan_projects(),
                        KeyCode::Char('r') => app.start_editing(),
                        KeyCode::Char('s') => app.toggle_sort(),
                        KeyCode::Char('/') => app.start_search(),
                        KeyCode::Char('<') => app.go_to_top(),
                        KeyCode::Char('>') => app.go_to_bottom(),
                        KeyCode::Char('e') => app.expand_folder(),
                        KeyCode::Char('w') => app.collapse_folder(),
                        KeyCode::Enter => app.toggle_folder(),
                        // Arrow keys
                        KeyCode::Down | KeyCode::Char('j') => app.next_project(),
                        KeyCode::Up | KeyCode::Char('k') => app.previous_project(),
                        // Page navigation (Ctrl+U and Ctrl+D like vim)
                        KeyCode::Char('u') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => app.page_up(),
                        KeyCode::Char('d') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => app.page_down(),
                        // Vim navigation (h/l for future horizontal navigation)
                        KeyCode::Char('h') => {} // Left - no action for now
                        KeyCode::Char('l') => {} // Right - no action for now
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => app.save_port(),
                        KeyCode::Esc => app.cancel_editing(),
                        KeyCode::Char(c) => app.port_input.push(c),
                        KeyCode::Backspace => {
                            app.port_input.pop();
                        }
                        _ => {}
                    },
                    InputMode::Searching => match key.code {
                        KeyCode::Enter => app.confirm_search(),
                        KeyCode::Esc => app.cancel_search(),
                        KeyCode::Char(c) => {
                            app.search_input.push(c);
                            app.filter_projects();
                        }
                        KeyCode::Backspace => {
                            app.search_input.pop();
                            app.filter_projects();
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

