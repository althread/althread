use std::{collections::HashMap, path::{Path, PathBuf}};
use super::filesystem::FileSystem;

use crate::{ast::import_block::{ImportBlock, ImportItem, ImportPath}, error::{AlthreadError, AlthreadResult, ErrorType}};


#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub name: String,
    pub path: PathBuf,
    pub alias: Option<String>,
    pub module_type: ModuleType,
}

#[derive(Debug, Clone)]
pub enum ModuleType {
    /// Single file module
    File,
    /// Directory module with sub-modules
    Directory { sub_modules: Vec<SubModule> },
}

#[derive(Debug, Clone)]
pub struct SubModule {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ModuleResolver<F: FileSystem> {
    pub current_file_dir: PathBuf,
    pub project_root: PathBuf,
    pub resolved_modules: HashMap<String, ResolvedModule>,
    pub filesystem: F,
}

impl <F: FileSystem> ModuleResolver<F> {
    pub fn new(current_file: &Path, filesystem: F) -> Self {
        let current_file_dir = current_file.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        // Find project root by looking for alt.toml file
        let project_root = Self::find_project_root(&current_file_dir)
            .unwrap_or_else(|| current_file_dir.clone());

        Self {
            current_file_dir,
            project_root,
            resolved_modules: HashMap::new(),
            filesystem
        }
    }

    fn find_project_root(start_dir: &Path) -> Option<PathBuf> {
        let mut current = start_dir.to_path_buf();
        
        loop {
            let alt_toml_path = current.join("alt.toml");
            if alt_toml_path.exists() {
                return Some(current);
            }
            
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
        
        None
    }

    pub fn resolve_imports(&mut self, import_block: &ImportBlock) -> AlthreadResult<()> {
        for import_item in &import_block.imports {
            let resolved = self.resolve_import_item(&import_item.value)?;
            let access_name = resolved.alias.clone().unwrap_or(resolved.name.clone());

            self.resolved_modules.insert(
                access_name,
                resolved
            );
        }

        Ok(())
    }

    fn resolve_import_item(&self, item: &ImportItem) -> AlthreadResult<ResolvedModule> {
        let (module_path, module_type) = self.resolve_path(&item.path)?;
        let name = item.path.last_segment().to_string();
        let alias = item.alias.as_ref().map(|alias| alias.value.value.clone());

        Ok(ResolvedModule {
            name, 
            path: module_path,
            alias,
            module_type,
        })
    }

    fn resolve_path(&self, import_path: &ImportPath) -> AlthreadResult<(PathBuf, ModuleType)> {
        let relative_path_str = import_path.segments.join("/");
        
        // First, try to resolve as a local path
        let mut local_path = self.current_file_dir.clone();
        local_path.push(&relative_path_str);
        
        if let Some(resolved) = self.resolve_local_path(&local_path)? {
            return Ok(resolved);
        }

        // If not found locally, check if it might be a remote dependency
        if Self::is_remote_import(&relative_path_str) {
            return self.resolve_remote_dependency(import_path);
        }

        Err(AlthreadError::new(
            ErrorType::ModuleNotFound,
            None, // Use None instead of Some(Pos::default()) to avoid the line number issue
            format!("Module '{}' not found in path '{}'", import_path.to_string(), self.current_file_dir.display())
        ))
    }

    fn resolve_local_path(&self, base_path: &PathBuf) -> AlthreadResult<Option<(PathBuf, ModuleType)>> {
        let file_path = base_path.with_extension("alt");
        let dir_path = base_path.clone();
        
        let file_exists = self.filesystem.is_file(&file_path);
        let dir_exists = self.filesystem.is_dir(&dir_path);
        
        match (file_exists, dir_exists) {
            (true, true) => {
                // Both exist: use file and warn user
                eprintln!("Warning: Both '{}' and '{}' exist. Using the file. Consider renaming one to avoid confusion.", 
                         file_path.display(), dir_path.display());
                let canonical_path = self.filesystem.canonicalize(&file_path)?;
                Ok(Some((canonical_path, ModuleType::File)))
            },
            (true, false) => {
                // Only file exists
                let canonical_path = self.filesystem.canonicalize(&file_path)?;
                Ok(Some((canonical_path, ModuleType::File)))
            },
            (false, true) => {
                // Only directory exists
                let sub_modules = self.discover_sub_modules(&dir_path)?;
                let canonical_path = self.filesystem.canonicalize(&dir_path)?;
                Ok(Some((canonical_path, ModuleType::Directory { sub_modules })))
            },
            (false, false) => {
                // Neither exists
                Ok(None)
            }
        }
    }

    fn discover_sub_modules(&self, dir_path: &PathBuf) -> AlthreadResult<Vec<SubModule>> {
        let mut sub_modules = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "alt") {
                        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                            sub_modules.push(SubModule {
                                name: name.to_string(),
                                path: path.clone(),
                            });
                        }
                    }
                }
            }
        }
        
        // Sort for consistent ordering
        sub_modules.sort_by(|a, b| a.name.cmp(&b.name));
        
        Ok(sub_modules)
    }

    fn is_remote_import(path: &str) -> bool {
        // Check if it looks like a remote import:
        // 1. Contains dots (suggesting a domain like github.com)
        // 2. Exclude relative paths
        path.contains('.') && !path.starts_with("./") && !path.starts_with("../")
    }

    fn resolve_remote_dependency(&self, import_path: &ImportPath) -> AlthreadResult<(PathBuf, ModuleType)> {
        let import_str = import_path.to_string();
        
        // For web environment, try to find in virtual filesystem first
        if let Some(result) = self.find_virtual_dependency(&import_str) {
            return Ok(result);
        }
        
        // CLI environment: check if project has alt.toml
        let alt_toml_path = self.project_root.join("alt.toml");
        if !alt_toml_path.exists() {
            return Err(AlthreadError::new(
                ErrorType::ModuleNotFound,
                None,
                format!(
                    "Cannot import remote module '{}'. \
                     Run 'althread add {}' to add the dependency (this will create alt.toml automatically).",
                    import_str, import_str
                )
            ));
        }

        // Check if the import is actually listed in dependencies
        if !self.is_dependency_declared(&import_str) {
            return Err(AlthreadError::new(
                ErrorType::ModuleNotFound,
                None,
                format!(
                    "Remote module '{}' not found in dependencies. \
                     Run 'althread add {}' to add it to alt.toml.",
                    import_str, import_str
                )
            ));
        }

        // Try to resolve from cache
        if let Some((cached_path, module_type)) = self.find_cached_dependency(&import_str) {
            return Ok((cached_path, module_type));
        }

        // Not in cache, suggest installing
        Err(AlthreadError::new(
            ErrorType::ModuleNotFound,
            None,
            format!(
                "Remote dependency '{}' not found in cache. \
                 Run 'althread install' to fetch dependencies.",
                import_str
            )
        ))
    }

    fn is_dependency_declared(&self, import_str: &str) -> bool {
        // Try to load alt.toml and check if the import is declared
        let alt_toml_path = self.project_root.join("alt.toml");
        if let Ok(content) = std::fs::read_to_string(&alt_toml_path) {
            // For hierarchical imports, we need to check if the base package is declared
            // e.g., "github.com/user/repo/algebra/floats" should match "github.com/user/repo"
            let import_parts: Vec<&str> = import_str.split('/').collect();
            
            if import_parts.len() >= 3 {
                // Check if the base package (first 3 parts) is declared
                let base_package = import_parts[0..3].join("/");
                if content.contains(&base_package) {
                    return true;
                }
            }
            
            // Also check if the full import string is declared (for backward compatibility)
            content.contains(import_str)
        } else {
            false
        }
    }

    fn find_cached_dependency(&self, import_str: &str) -> Option<(PathBuf, ModuleType)> {
        // For hierarchical imports, we need to extract the base package path
        // e.g., "github.com/user/repo/algebra/floats" -> "github.com/user/repo"
        let import_parts: Vec<&str> = import_str.split('/').collect();
        
        if import_parts.len() < 3 {
            return None;
        }
        
        // First, try to find in virtual filesystem (for web environment)
        if let Some(result) = self.find_virtual_dependency(import_str) {
            return Some(result);
        }
        
        // Fallback to standard cache directory (for CLI environment)
        let home_dir = std::env::var("HOME").ok()?;
        let cache_dir = std::path::PathBuf::from(home_dir).join(".althread/cache");
        
        let base_package = import_parts[0..3].join("/");
        let sanitized_base = base_package.replace("://", "/");
        let dep_base_path = cache_dir.join(&sanitized_base);
        
        // Look for any version directory (we'll implement proper version resolution later)
        if let Ok(entries) = std::fs::read_dir(&dep_base_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let version_path = entry.path();
                    if version_path.is_dir() {
                        // Try to resolve the hierarchical import within the cached dependency
                        if let Some(resolved) = self.resolve_cached_import(&version_path, import_str) {
                            return Some(resolved);
                        }
                    }
                }
            }
        }
        
        None
    }
    
    fn find_virtual_dependency(&self, import_str: &str) -> Option<(PathBuf, ModuleType)> {
        // Look for dependencies in the virtual filesystem under deps/
        // e.g., "github.com/user/repo/algebra/floats" maps to "deps/github_com_user_repo/algebra/floats.alt"
        
        let import_parts: Vec<&str> = import_str.split('/').collect();
        
        if import_parts.len() < 3 {
            return None;
        }
        
        let base_package = import_parts[0..3].join("/");
        
        // Convert package name to filesystem-safe name (replace special chars with underscores)
        let sanitized_package = base_package.replace(|c: char| !c.is_alphanumeric(), "_");
        
        let has_hierarchical_part = import_parts.len() > 3;
        
        let target_path = if has_hierarchical_part {
            let hierarchical_part = import_parts[3..].join("/");
            std::path::PathBuf::from(format!("deps/{}/{}", sanitized_package, hierarchical_part))
        } else {
            // Root level import, look for main.alt
            std::path::PathBuf::from(format!("deps/{}/main", sanitized_package))
        };
        
        // Check if it's a file or directory in the virtual filesystem
        let file_path = target_path.with_extension("alt");
        let dir_path = target_path.clone();
        
        let is_file = self.filesystem.is_file(&file_path);
        let is_dir = self.filesystem.is_dir(&dir_path);
        
        if is_file && is_dir {
            // Both exist: prefer file and warn
            eprintln!("Warning: Both file '{}' and directory '{}' exist. Using file.", 
                     file_path.display(), dir_path.display());
            return Some((file_path, ModuleType::File));
        } else if is_file {
            return Some((file_path, ModuleType::File));
        } else if is_dir {
            // Directory module: collect all .alt files as sub-modules
            let sub_modules = self.collect_virtual_submodules(&dir_path);
            return Some((dir_path, ModuleType::Directory { sub_modules }));
        }
        
        None
    }
    
    fn collect_virtual_submodules(&self, _dir_path: &std::path::Path) -> Vec<SubModule> {
        // For virtual filesystem, we need to look for files that start with the directory path
        // This is a simplified approach since we can't actually list directory contents
        // in a virtual filesystem without more complex iteration
        
        // For now, return an empty vector - this could be enhanced to actually
        // discover submodules by iterating through the virtual filesystem keys
        Vec::new()
    }

    fn resolve_cached_import(&self, cached_package_path: &PathBuf, import_str: &str) -> Option<(PathBuf, ModuleType)> {
        // Parse the import to extract the hierarchical path
        // e.g., "github.com/user/repo/algebra/floats" -> "algebra/floats"
        let import_parts: Vec<&str> = import_str.split('/').collect();
        
        if import_parts.len() < 3 {
            // Not a valid remote import with hierarchical path
            return None;
        }
        
        let package_part = import_parts[0..3].join("/"); // e.g., "github.com/user/repo"
        let has_hierarchical_part = import_parts.len() > 3;
        
        let target_path = if has_hierarchical_part {
            let hierarchical_part = import_parts[3..].join("/"); // e.g., "algebra/floats"
            cached_package_path.join(hierarchical_part)
        } else {
            // Root level import, look for main.alt
            cached_package_path.join("main.alt")
        };
        
        // Check if it's a file or directory
        let file_path = target_path.with_extension("alt");
        let dir_path = target_path.clone();
        
        if file_path.exists() && dir_path.exists() {
            // Both exist: prefer file and warn
            eprintln!("Warning: Both '{}' and '{}' exist in cached package '{}'. Using the file.", 
                     file_path.display(), dir_path.display(), package_part);
            return Some((file_path, ModuleType::File));
        } else if file_path.exists() {
            // Only file exists
            return Some((file_path, ModuleType::File));
        } else if dir_path.exists() {
            // Only directory exists
            if let Ok(sub_modules) = self.discover_sub_modules(&dir_path) {
                return Some((dir_path, ModuleType::Directory { sub_modules }));
            }
        }
        
        // If we're at root level and main.alt doesn't exist, check if it's a directory package
        if !has_hierarchical_part && cached_package_path.is_dir() {
            if let Ok(sub_modules) = self.discover_sub_modules(cached_package_path) {
                return Some((cached_package_path.clone(), ModuleType::Directory { sub_modules }));
            }
        }
        
        None
    }
}