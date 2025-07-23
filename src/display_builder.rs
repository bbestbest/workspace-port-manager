/*!
 * Display Builder Module
 * 
 * This module follows the Single Responsibility Principle (SRP).
 * It has ONE job: converting project data into display items for the UI.
 * 
 * Key Rust concepts demonstrated:
 * - Collections: BTreeMap for sorted data, HashSet for unique items
 * - Iterators and functional programming patterns
 * - String formatting and manipulation
 * - Sorting with custom comparators
 * - Data transformation pipelines
 */

use std::collections::{BTreeMap, HashSet};
use crate::{
    navigation::DisplayItem,
    project_scanner::ProjectInfo,
};

/// Display Builder - converts projects into UI-ready display items
/// 
/// This struct follows SRP: it only handles display list construction.
/// It doesn't know about navigation, UI rendering, or file operations.
pub struct DisplayBuilder;

impl DisplayBuilder {
    /// Creates a new display builder
    pub fn new() -> Self {
        Self
    }
    
    /// Builds display items from projects
    /// 
    /// This is the main method that converts a list of projects into
    /// a hierarchical display list with folder headers and project items.
    /// 
    /// Parameters:
    /// - projects: The list of all projects
    /// - collapsed_folders: Set of folder names that should be collapsed
    /// - sort_by_port: Whether to sort by port (true) or name (false)
    /// 
    /// Returns a Vec<DisplayItem> ready for display
    pub fn build_display_items(
        &self,
        projects: &[ProjectInfo],
        collapsed_folders: &HashSet<String>,
        sort_by_port: bool,
    ) -> Vec<DisplayItem> {
        let mut display_items = Vec::new();
        
        // Group projects by their parent folder
        // BTreeMap keeps folders sorted alphabetically
        let mut grouped_projects: BTreeMap<String, Vec<(usize, &ProjectInfo)>> = BTreeMap::new();
        
        // Group projects by parent folder with their original indices
        for (index, project) in projects.iter().enumerate() {
            grouped_projects
                .entry(project.parent_folder.clone())  // Get or create entry
                .or_insert_with(Vec::new)              // Create empty vec if new
                .push((index, project));               // Add project with index
        }
        
        // Build display items for each folder
        for (folder_name, mut folder_projects) in grouped_projects {
            // Add folder header
            let header_item = self.create_folder_header(&folder_name, collapsed_folders);
            display_items.push(header_item);
            
            // Only add projects if folder is not collapsed
            if !collapsed_folders.contains(&folder_name) {
                // Sort projects within the folder
                self.sort_projects(&mut folder_projects, sort_by_port);
                
                // Create display items for each project
                for (original_index, project) in folder_projects {
                    let project_item = self.create_project_item(project, original_index, &folder_name);
                    display_items.push(project_item);
                }
            }
        }
        
        display_items
    }
    
    /// Filters display items based on search query
    /// 
    /// This method implements search functionality by filtering both
    /// folder headers and project items based on the search term.
    pub fn filter_display_items(
        &self,
        display_items: &[DisplayItem],
        search_query: &str,
    ) -> Vec<DisplayItem> {
        if search_query.is_empty() {
            return display_items.to_vec();  // Return copy if no search
        }
        
        let search_lower = search_query.to_lowercase();
        
        // Filter items based on search criteria
        display_items
            .iter()
            .filter(|item| self.item_matches_search(item, &search_lower))
            .cloned()  // Clone each item (since we're returning owned data)
            .collect() // Collect into new Vec
    }
    
    // === Private Helper Methods ===
    
    /// Creates a folder header display item
    /// 
    /// Folder headers show an expand/collapse indicator and folder icon.
    fn create_folder_header(&self, folder_name: &str, collapsed_folders: &HashSet<String>) -> DisplayItem {
        let is_collapsed = collapsed_folders.contains(folder_name);
        
        // Choose expand/collapse indicator
        let expand_indicator = if is_collapsed { "▶" } else { "▼" };
        
        // Format: "📂 ▼ foldername/"
        let content = format!("📂 {} {}/", expand_indicator, folder_name);
        
        DisplayItem {
            content,
            is_header: true,
            project_index: None,  // Headers don't reference specific projects
        }
    }
    
    /// Creates a project item display item
    /// 
    /// Project items show the project name and port status.
    fn create_project_item(
        &self,
        project: &ProjectInfo,
        original_index: usize,
        folder_name: &str,
    ) -> DisplayItem {
        // Format port information
        let port_text = match project.port {
            Some(port) => format!(":{}", port),  // ":3000"
            None => "─".to_string(),             // "─" when no port
        };
        
        // Remove folder prefix from project name for cleaner display
        let project_name = project
            .name
            .strip_prefix(&format!("{}/", folder_name))  // Remove "folder/" prefix
            .unwrap_or(&project.name);                   // Fall back to full name
        
        // Truncate long project names to fit display
        let max_name_width = 35;
        let truncated_name = if project_name.len() > max_name_width {
            format!("{}…", &project_name[..max_name_width.saturating_sub(1)])
        } else {
            project_name.to_string()
        };
        
        // Format: "  📁 project_name                    :3000"
        // The spacing ensures port numbers align vertically
        let content = format!("  📁 {:<35} {}", truncated_name, port_text);
        
        DisplayItem {
            content,
            is_header: false,
            project_index: Some(original_index),
        }
    }
    
    /// Sorts projects within a folder
    /// 
    /// This method demonstrates Rust's flexible sorting capabilities.
    /// We can sort by different criteria using closures.
    fn sort_projects(
        &self,
        projects: &mut [(usize, &ProjectInfo)],
        sort_by_port: bool,
    ) {
        if sort_by_port {
            // Sort by port number, then by name
            projects.sort_by(|(_, a), (_, b)| {
                // This is a custom comparator using pattern matching
                match (a.port, b.port) {
                    (Some(port_a), Some(port_b)) => port_a.cmp(&port_b),      // Both have ports
                    (Some(_), None) => std::cmp::Ordering::Less,              // Ports come first
                    (None, Some(_)) => std::cmp::Ordering::Greater,           // No port comes last
                    (None, None) => a.name.cmp(&b.name),                     // Both no port: sort by name
                }
            });
        } else {
            // Sort by name only
            projects.sort_by(|(_, a), (_, b)| a.name.cmp(&b.name));
        }
    }
    
    /// Checks if a display item matches the search query
    /// 
    /// This implements the search logic for both headers and project items.
    fn item_matches_search(&self, item: &DisplayItem, search_lower: &str) -> bool {
        if item.is_header {
            // For headers, search in folder name
            self.header_matches_search(&item.content, search_lower)
        } else {
            // For project items, search in project name and port
            self.project_matches_search(&item.content, search_lower)
        }
    }
    
    /// Checks if a folder header matches the search
    fn header_matches_search(&self, header_content: &str, search_lower: &str) -> bool {
        // Extract folder name from "📂 ▼ foldername/"
        let folder_name = header_content
            .strip_prefix("📂 ")           // Remove emoji and space
            .unwrap_or(header_content)     // Fallback to full content
            .split_whitespace()            // Split by whitespace
            .nth(1)                        // Get second part (folder name)
            .unwrap_or("")                 // Fallback to empty string
            .trim_end_matches('/')         // Remove trailing slash
            .to_lowercase();               // Convert to lowercase for comparison
        
        folder_name.contains(search_lower)
    }
    
    /// Checks if a project item matches the search
    fn project_matches_search(&self, project_content: &str, search_lower: &str) -> bool {
        // Search in the entire project line (name and port)
        let content_lower = project_content.to_lowercase();
        content_lower.contains(search_lower)
    }
}

/// Default implementation for DisplayBuilder
impl Default for DisplayBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/*
 * Key Rust Concepts Explained:
 * 
 * 1. **Collections**:
 *    - BTreeMap<K, V>: Sorted map, keys are always in order
 *    - HashSet<T>: Set of unique values, fast lookup
 *    - Vec<T>: Dynamic array, most common collection
 * 
 * 2. **Iterator Patterns**:
 *    - .iter() creates an iterator over references
 *    - .enumerate() adds indices: (index, item)
 *    - .filter() keeps items that match a condition
 *    - .cloned() converts &T to T (makes copies)
 *    - .collect() consumes iterator into a collection
 * 
 * 3. **String Operations**:
 *    - .strip_prefix() removes prefix if present
 *    - .split_whitespace() splits on any whitespace
 *    - .nth(1) gets the nth item from iterator
 *    - format!() macro for string formatting
 * 
 * 4. **Sorting**:
 *    - .sort_by() with custom comparator function
 *    - std::cmp::Ordering enum for comparison results
 *    - Pattern matching on tuples for complex comparisons
 * 
 * 5. **Method Chaining**:
 *    - Rust encourages chaining operations
 *    - Each method returns something the next can use
 *    - Creates readable data transformation pipelines
 * 
 * 6. **Pattern Matching**:
 *    - match expressions handle all cases
 *    - Destructuring tuples and enums
 *    - Option handling with Some/None patterns
 */