/*!
 * Port Management Module
 * 
 * This module follows the Single Responsibility Principle (SRP).
 * It has ONE job: managing port assignments in .env files.
 * 
 * Key Rust concepts demonstrated:
 * - Error handling with Result<T, E>
 * - File I/O operations
 * - String manipulation and parsing
 * - Pattern matching for error handling
 * - Method chaining with combinators
 */

use std::{fs, path::Path};
use crate::project_scanner::ProjectInfo;

/// Port Manager - handles reading and writing port configurations
/// 
/// This struct follows SRP: it only handles port file operations.
/// It doesn't know about UI, navigation, or project scanning.
pub struct PortManager {
    /// Root workspace directory for resolving project paths
    workspace_root: std::path::PathBuf,
}

/// Custom error types for port management operations
/// 
/// Enums in Rust can carry data in each variant.
/// This allows us to provide specific error information.
#[derive(Debug)]
pub enum PortError {
    /// File system error (file not found, permission denied, etc.)
    FileSystem(std::io::Error),
    /// Invalid port number (not a valid u16)
    InvalidPort(String),
    /// Project not found
    ProjectNotFound,
}

/// Implement Display trait for our error type
/// 
/// This allows our errors to be printed nicely.
/// The Display trait is like toString() in other languages.
impl std::fmt::Display for PortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortError::FileSystem(err) => write!(f, "File system error: {}", err),
            PortError::InvalidPort(port_str) => write!(f, "Invalid port number: '{}'", port_str),
            PortError::ProjectNotFound => write!(f, "Project not found"),
        }
    }
}

/// Implement Error trait for our error type
/// 
/// This allows our error to work with Rust's error handling ecosystem.
impl std::error::Error for PortError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PortError::FileSystem(err) => Some(err),
            _ => None,
        }
    }
}

/// Convert io::Error to our PortError automatically
/// 
/// The From trait allows automatic conversion with the ? operator.
/// When we use ? on an io::Error, it automatically becomes a PortError.
impl From<std::io::Error> for PortError {
    fn from(err: std::io::Error) -> Self {
        PortError::FileSystem(err)
    }
}

impl PortManager {
    /// Creates a new PortManager
    /// 
    /// This follows the same pattern as other modules - simple constructor.
    pub fn new() -> Self {
        let workspace_root = dirs::home_dir()
            .map(|home| home.join("Workspace"))
            .unwrap_or_else(|| Path::new("~/Workspace").to_path_buf());
        
        Self {
            workspace_root,
        }
    }
    
    /// Sets a port for a specific project
    /// 
    /// This method demonstrates Rust's error handling with Result<T, E>.
    /// Result is an enum with two variants: Ok(success_value) and Err(error_value).
    /// 
    /// Parameters:
    /// - project: The project to update
    /// - port: The new port number
    /// 
    /// Returns:
    /// - Ok(()) if successful (unit type - like void but explicit)
    /// - Err(PortError) if something went wrong
    pub fn set_port(&self, project: &ProjectInfo, port: u16) -> Result<(), PortError> {
        // Build path to project directory
        let project_path = self.workspace_root.join(&project.name);
        
        // Ensure project directory exists
        if !project_path.exists() {
            return Err(PortError::ProjectNotFound);
        }
        
        // Path to .env file within project
        let env_file = project_path.join(".env");
        
        // Read existing .env content (or empty string if file doesn't exist)
        let existing_content = if env_file.exists() {
            fs::read_to_string(&env_file)?  // ? operator: return error if read fails
        } else {
            String::new()  // Empty string for new .env files
        };
        
        // Update or add PORT= line
        let new_content = self.update_port_in_content(&existing_content, port);
        
        // Write updated content back to file
        fs::write(&env_file, new_content)?;  // ? operator again
        
        Ok(())  // Success - return unit type wrapped in Ok
    }
    
    /// Gets the current port for a project
    /// 
    /// Returns Option<u16> because the project might not have a port set.
    pub fn get_port(&self, project: &ProjectInfo) -> Option<u16> {
        let project_path = self.workspace_root.join(&project.name);
        let env_file = project_path.join(".env");
        
        // Early return if file doesn't exist
        if !env_file.exists() {
            return None;
        }
        
        // Try to read file content
        let content = fs::read_to_string(&env_file).ok()?;  // ? with Option
        
        // Search for PORT= line
        self.extract_port_from_content(&content)
    }
    
    /// Validates a port number string
    /// 
    /// This is a utility method that checks if a string represents a valid port.
    pub fn validate_port(port_str: &str) -> Result<u16, PortError> {
        port_str.parse::<u16>()
            .map_err(|_| PortError::InvalidPort(port_str.to_string()))
    }
    
    /// Updates all projects with their current port information
    /// 
    /// This method modifies the projects vector in place.
    /// The '&mut' parameter allows us to modify the vector's contents.
    pub fn refresh_port_info(&self, projects: &mut [ProjectInfo]) {
        for project in projects.iter_mut() {  // iter_mut() gives mutable references
            project.port = self.get_port(project);
        }
    }
    
    /// Removes port setting from a project
    /// 
    /// This deletes the PORT= line from the .env file.
    pub fn remove_port(&self, project: &ProjectInfo) -> Result<(), PortError> {
        let project_path = self.workspace_root.join(&project.name);
        let env_file = project_path.join(".env");
        
        // If .env doesn't exist, nothing to remove
        if !env_file.exists() {
            return Ok(());
        }
        
        let existing_content = fs::read_to_string(&env_file)?;
        let new_content = self.remove_port_from_content(&existing_content);
        
        // If content is now empty, we could delete the file
        // But it's safer to just write empty content
        fs::write(&env_file, new_content)?;
        
        Ok(())
    }
    
    // === Private Helper Methods ===
    
    /// Updates PORT= line in .env file content
    /// 
    /// This method handles three cases:
    /// 1. PORT= line exists - replace it
    /// 2. PORT= line doesn't exist - add it
    /// 3. Multiple PORT= lines - replace first, remove others
    fn update_port_in_content(&self, content: &str, port: u16) -> String {
        let mut lines: Vec<&str> = content.lines().collect();
        let mut port_found = false;
        let port_line = format!("PORT={}", port);
        
        // First pass: replace existing PORT= lines
        for line in lines.iter_mut() {
            if line.starts_with("PORT=") {
                if !port_found {
                    // Replace the first PORT= line
                    *line = &port_line;
                    port_found = true;
                } else {
                    // Mark additional PORT= lines for removal
                    *line = "";
                }
            }
        }
        
        // If no PORT= line was found, add one
        if !port_found {
            lines.push(&port_line);
        }
        
        // Filter out empty lines (removed duplicate PORT= lines)
        // and rejoin with newlines
        lines
            .into_iter()
            .filter(|line| !line.is_empty())  // Remove empty lines
            .collect::<Vec<_>>()              // Collect into vector
            .join("\n")                       // Join with newlines
            + "\n"                            // Add final newline
    }
    
    /// Removes PORT= line from .env file content
    fn remove_port_from_content(&self, content: &str) -> String {
        content
            .lines()                          // Split into lines
            .filter(|line| !line.starts_with("PORT="))  // Keep non-PORT lines
            .collect::<Vec<_>>()              // Collect into vector
            .join("\n")                       // Join with newlines
    }
    
    /// Extracts port number from .env file content
    /// 
    /// This searches through all lines looking for PORT= entries.
    fn extract_port_from_content(&self, content: &str) -> Option<u16> {
        for line in content.lines() {
            if line.starts_with("PORT=") {
                // Extract the part after "PORT="
                let port_str = line.trim_start_matches("PORT=").trim();
                
                // Try to parse as number
                if let Ok(port) = port_str.parse::<u16>() {
                    return Some(port);
                }
            }
        }
        None
    }
}

/// Default implementation for PortManager
impl Default for PortManager {
    fn default() -> Self {
        Self::new()
    }
}

/*
 * Key Rust Concepts Explained:
 * 
 * 1. **Error Handling**:
 *    - Result<T, E> for operations that can fail
 *    - Custom error types with enum variants
 *    - ? operator for early return on errors
 *    - From trait for automatic error conversion
 * 
 * 2. **Option Type**:
 *    - Option<T> for values that might not exist
 *    - Methods like .ok() convert Result to Option
 *    - ? operator works with both Result and Option
 * 
 * 3. **String Operations**:
 *    - &str (string slice) vs String (owned string)
 *    - String manipulation with lines(), filter(), collect()
 *    - Iterator chaining for data processing
 * 
 * 4. **File I/O**:
 *    - fs::read_to_string() and fs::write() for simple file operations
 *    - Path operations with join() and exists()
 *    - Cross-platform path handling
 * 
 * 5. **Borrowing and Ownership**:
 *    - &self for read-only methods
 *    - &mut for methods that modify data
 *    - Moving vs borrowing data
 * 
 * 6. **Pattern Matching**:
 *    - match expressions for handling different cases
 *    - if let for simple pattern matches
 *    - Extracting data from enum variants
 */