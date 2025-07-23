/*!
 * Project Scanner Module
 * 
 * This module follows the Single Responsibility Principle (SRP) from SOLID.
 * It has ONE job: scanning directories and detecting development projects.
 * 
 * Key Rust concepts used here:
 * - Modules: Rust's way of organizing code into separate files/folders
 * - Structs: Custom data types that group related data together
 * - impl blocks: Where we define methods (functions) for our structs
 * - Option<T>: Rust's way of handling values that might not exist (no null!)
 * - Vec<T>: Dynamic arrays (like ArrayList in Java or list in Python)
 * - Path/PathBuf: Rust's cross-platform way of handling file system paths
 */

use std::{fs, path::Path};
use serde::{Deserialize, Serialize};

/// Represents information about a development project
/// 
/// In Rust, structs are like classes but without methods by default.
/// The #[derive(...)] attributes automatically generate common functionality:
/// - Debug: Allows printing the struct for debugging
/// - Clone: Allows making copies of the struct
/// - Serialize/Deserialize: Allows converting to/from JSON (from serde crate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// The project's name/path relative to workspace
    pub name: String,
    
    /// The port number if found in .env file
    /// Option<u16> means this can be Some(port_number) or None
    /// u16 is an unsigned 16-bit integer (0-65535, perfect for port numbers)
    pub port: Option<u16>,
    
    /// Full file system path to the project
    pub path: String,
    
    /// The parent folder name (like "mac", "agm", "azay")
    pub parent_folder: String,
}

/// Project Scanner - responsible for finding development projects
/// 
/// This struct follows SRP: it only handles project discovery.
/// The 'pub' keyword makes it available to other modules.
pub struct ProjectScanner {
    /// The root workspace directory to scan
    workspace_root: std::path::PathBuf,
}

/// Implementation block - where we define methods for ProjectScanner
/// 
/// In Rust, methods are defined separately from the struct definition.
/// This is different from languages like Java/C# where methods are inside classes.
impl ProjectScanner {
    /// Constructor method - creates a new ProjectScanner
    /// 
    /// In Rust, constructors are just regular functions that return the struct.
    /// The 'pub' makes this method callable from other modules.
    /// 'Self' is a shorthand for 'ProjectScanner' - it refers to the current type.
    pub fn new() -> Self {
        // dirs::home_dir() returns Option<PathBuf> - it might fail!
        // .map() transforms the value if it exists
        // .unwrap_or_else() provides a default if it doesn't exist
        let workspace_root = dirs::home_dir()
            .map(|home| home.join("Workspace"))  // If home exists, append "Workspace"
            .unwrap_or_else(|| Path::new("~/Workspace").to_path_buf()); // Default fallback
        
        Self {
            workspace_root,
        }
    }
    
    /// Scans for all development projects in the workspace
    /// 
    /// Returns a Vec<ProjectInfo> - a vector (dynamic array) of project information.
    /// The '&self' parameter means this method borrows the struct (doesn't take ownership).
    /// This is Rust's way of avoiding unnecessary copying.
    pub fn scan_projects(&self) -> Vec<ProjectInfo> {
        let mut projects = Vec::new(); // Create empty vector - 'mut' means mutable
        
        // Check if workspace directory exists before scanning
        if self.workspace_root.exists() {
            // Start recursive scanning from workspace root
            self.scan_recursive(&self.workspace_root, &mut projects);
        }
        
        projects
    }
    
    /// Recursively scans directories for projects
    /// 
    /// This is a private method (no 'pub') - only this module can call it.
    /// '&self' borrows the struct, '&mut Vec<ProjectInfo>' borrows the vector mutably.
    /// Mutable borrowing means we can modify the vector's contents.
    fn scan_recursive(&self, current_dir: &Path, projects: &mut Vec<ProjectInfo>) {
        // Try to read directory contents
        // fs::read_dir() returns Result<ReadDir, Error> - it can fail!
        // 'if let Ok(entries)' is pattern matching - only runs if successful
        if let Ok(entries) = fs::read_dir(current_dir) {
            // Iterate through each entry in the directory
            for entry in entries {
                // Each entry is also a Result - it can fail too!
                if let Ok(entry) = entry {
                    let path = entry.path();
                    
                    // Only process directories (skip files)
                    if path.is_dir() {
                        // Skip hidden directories and common build folders
                        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                            if self.should_skip_directory(dir_name) {
                                continue; // Skip this iteration, go to next
                            }
                        }
                        
                        // Check if this directory is a development project
                        if self.is_working_project(&path) {
                            // Create ProjectInfo and add to our collection
                            if let Some(project_info) = self.create_project_info(&path) {
                                projects.push(project_info);
                            }
                        } else {
                            // Not a project, but might contain projects - recurse deeper
                            let depth = self.calculate_depth(&path);
                            if depth < 3 { // Limit recursion depth to avoid infinite loops
                                self.scan_recursive(&path, projects);
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Determines if a directory should be skipped during scanning
    /// 
    /// Returns 'bool' - true if we should skip, false if we should process.
    /// The '&self' parameter isn't used here, but Rust requires it for methods.
    fn should_skip_directory(&self, dir_name: &str) -> bool {
        // List of directories to skip
        // These are common folders that won't contain projects we care about
        matches!(dir_name, 
            name if name.starts_with('.') ||  // Hidden directories (.git, .vscode, etc.)
            name == "node_modules" ||         // JavaScript dependencies
            name == "target" ||               // Rust build output
            name == "build" ||                // General build output
            name == "dist" ||                 // Distribution files
            name == "__pycache__"             // Python cache
        )
    }
    
    /// Checks if a directory contains development project files
    /// 
    /// This method looks for common project files that indicate a working project.
    fn is_working_project(&self, project_path: &Path) -> bool {
        // Array of file names that indicate a development project
        // These are configuration files for different programming languages/tools
        let project_files = [
            "package.json",      // Node.js/JavaScript
            "Cargo.toml",        // Rust
            "pom.xml",           // Java (Maven)
            "build.gradle",      // Java (Gradle)
            "requirements.txt",  // Python
            "setup.py",          // Python
            "pyproject.toml",    // Modern Python
            "go.mod",            // Go
            "composer.json",     // PHP
            "Gemfile",           // Ruby
            "mix.exs",           // Elixir
            "pubspec.yaml",      // Dart/Flutter
            "CMakeLists.txt",    // C/C++
            "Makefile",          // C/C++/others
            "docker-compose.yml", // Docker
            "docker-compose.yaml", // Docker
            "Dockerfile",        // Docker
            ".gitignore",        // Git (most projects have this)
        ];
        
        // Try to read the directory
        if let Ok(entries) = fs::read_dir(project_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let file_name = entry.file_name();
                    if let Some(name) = file_name.to_str() {
                        // Check for .sln files (Visual Studio solutions)
                        if name.ends_with(".sln") || project_files.contains(&name) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
    
    /// Creates a ProjectInfo struct from a directory path
    /// 
    /// Returns Option<ProjectInfo> because this operation might fail.
    fn create_project_info(&self, project_path: &Path) -> Option<ProjectInfo> {
        // Calculate relative path from workspace root
        let relative_path = project_path
            .strip_prefix(&self.workspace_root)  // Remove workspace root from path
            .ok()?                               // '?' operator: return None if this fails
            .to_string_lossy()                   // Convert to string (might lose some Unicode)
            .to_string();
        
        // Extract parent folder name (first directory in relative path)
        let parent_folder = relative_path
            .split('/')                          // Split path by '/'
            .next()                              // Get first part
            .unwrap_or("other")                  // Default to "other" if none
            .to_string();
        
        // Detect port from .env file (if it exists)
        let port = self.detect_project_port(project_path);
        
        // Create and return the ProjectInfo struct
        Some(ProjectInfo {
            name: relative_path.clone(),
            port,
            path: project_path.to_string_lossy().to_string(),
            parent_folder,
        })
    }
    
    /// Calculates directory depth relative to workspace root
    /// 
    /// This helps us limit how deep we recurse to avoid performance issues.
    fn calculate_depth(&self, path: &Path) -> usize {
        path.strip_prefix(&self.workspace_root)
            .map(|p| p.components().count())     // Count path components
            .unwrap_or(0)                        // Default to 0 if calculation fails
    }
    
    /// Detects the port number from a project's .env file
    /// 
    /// Many development projects store their port in environment files.
    /// This method looks for "PORT=1234" lines in .env files.
    fn detect_project_port(&self, project_path: &Path) -> Option<u16> {
        let env_file = project_path.join(".env");  // Create path to .env file
        
        // Check if .env file exists
        if env_file.exists() {
            // Try to read the file contents
            if let Ok(content) = fs::read_to_string(&env_file) {
                // Look through each line for PORT= entries
                for line in content.lines() {
                    if line.starts_with("PORT=") {
                        // Extract the part after "PORT="
                        let port_str = line.trim_start_matches("PORT=");
                        // Try to parse as number
                        if let Ok(port) = port_str.parse::<u16>() {
                            return Some(port);
                        }
                    }
                }
            }
        }
        None // No port found
    }
}