use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::env::temp_dir;
use std::fs::remove_dir_all;
use std::str::from_utf8;
use crate::package::{Package, DependencyInfo, DependencySpec};
use crate::git::GitRepository;

/// Version resolution functionality
pub struct VersionResolver;

impl VersionResolver {
    /// Get the latest version from a git repository
    pub fn get_latest_version_from_repo(url: &str) -> Result<String, String> {
        let git_url = GitRepository::url_from_import_path(url);
        let temp_dir = Self::create_temp_dir(url, "latest")?;

        let git_repo = GitRepository::new(&git_url, temp_dir.clone());
        
        // Clone the repository
        git_repo.clone_if_needed()
            .map_err(|e| format!("Failed to clone repository {}: {}", url, e))?;

        // Get the latest version
        let latest_version = Self::get_latest_git_tag(&temp_dir)
            .unwrap_or_else(|_| {
                // If no tags, use the current commit hash
                Self::get_current_commit_hash(&temp_dir)
                    .unwrap_or_else(|_| "main".to_string())
            });

        // Clean up temp directory
        let _ = remove_dir_all(&temp_dir);

        Ok(latest_version)
    }

    /// Parse a version tag into a semver::Version
    pub fn parse_version_tag(tag: &str) -> Result<semver::Version, semver::Error> {
        let clean_tag = tag
            .strip_prefix("version-")
            .or_else(|| tag.strip_prefix("version"))
            .or_else(|| tag.strip_prefix("release-"))
            .or_else(|| tag.strip_prefix("release"))
            .or_else(|| tag.strip_prefix("v"))
            .or_else(|| tag.strip_prefix("V"))
            .unwrap_or(tag);

        semver::Version::parse(clean_tag)
    }

    /// Get the latest git tag from a repository
    pub fn get_latest_git_tag(repo_path: &Path) -> Result<String, String> {
        let repo = git2::Repository::open(repo_path)
            .map_err(|e| format!("Failed to open repository: {}", e))?;

        let mut tags = Vec::new();

        repo.tag_foreach(|oid, name| {
            if let Ok(name_str) = from_utf8(name) {
                let tag_name = name_str.strip_prefix("refs/tags/").unwrap_or(name_str);

                if let Ok(tag_obj) = repo.find_object(oid, Some(git2::ObjectType::Tag)) {
                    if let Some(tag) = tag_obj.as_tag() {
                        let target_oid = tag.target_id();
                        if let Ok(commit) = repo.find_commit(target_oid) {
                            let commit_time = commit.time().seconds();
                            tags.push((tag_name.to_string(), commit_time));
                        }
                    }
                } else if let Ok(commit) = repo.find_commit(oid) {
                    let commit_time = commit.time().seconds();
                    tags.push((tag_name.to_string(), commit_time));
                }
            }
            true
        }).map_err(|e| format!("Failed to iterate tags: {}", e))?;

        if tags.is_empty() {
            return Err("No tags found in repository".to_string());
        }

        // Sort by commit time (newest first)
        tags.sort_by(|a, b| b.1.cmp(&a.1));

        // Try to find semantic version tags first
        let mut semantic_tags: Vec<(String, semver::Version)> = Vec::new();
        for (tag_name, _) in &tags {
            if let Ok(version) = Self::parse_version_tag(tag_name) {
                semantic_tags.push((tag_name.clone(), version));
            }
        }

        if !semantic_tags.is_empty() {
            // Sort semantic versions (newest first)
            semantic_tags.sort_by(|a, b| b.1.cmp(&a.1));
            return Ok(semantic_tags[0].0.clone());
        }

        // Return the most recent tag if no semantic versions
        Ok(tags[0].0.clone())
    }

    /// Get the current commit hash from a repository
    pub fn get_current_commit_hash(repo_path: &Path) -> Result<String, String> {
        let repo = git2::Repository::open(repo_path)
            .map_err(|e| format!("Failed to open repository: {}", e))?;
        
        let head = repo.head()
            .map_err(|e| format!("Failed to get HEAD: {}", e))?;
        
        let commit = head.peel_to_commit()
            .map_err(|e| format!("Failed to get commit: {}", e))?;
        
        Ok(commit.id().to_string()[..8].to_string())
    }

    /// Create a temporary directory for git operations (private helper)
    fn create_temp_dir(url: &str, operation: &str) -> Result<PathBuf, String> {
        let temp_dir = temp_dir().join(format!("althread_{}_{}",
            operation,
            url.replace("/", "_").replace(".", "_")));

        // Clean up any existing temp directory
        if temp_dir.exists() {
            let _ = remove_dir_all(&temp_dir);
        }

        Ok(temp_dir)
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct ResolutionContext {
    pub cache_dir: PathBuf,
    pub resolved: HashMap<String, ResolvedDependency>,
    pub visiting: HashSet<String>, // For cycle detection
}

impl ResolutionContext {
    pub fn new() -> Self {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let cache_dir = PathBuf::from(home_dir).join(".althread/cache");
        
        Self {
            cache_dir,
            resolved: HashMap::new(),
            visiting: HashSet::new(),
        }
    }

    pub fn cache_path_for(&self, url: &str, version: &str) -> PathBuf {
        // Create a deterministic path structure
        // github.com/user/repo@v1.0.0 -> ~/.althread/cache/github.com/user/repo/v1.0.0
        let sanitized_url = url.replace("://", "/");
        self.cache_dir.join(&sanitized_url).join(version)
    }
}

pub struct DependencyResolver {
    context: ResolutionContext,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {
            context: ResolutionContext::new(),
        }
    }

    /// Resolve and install all dependencies for a package in a single pass
    pub fn resolve_and_install_dependencies(&mut self, package: &Package, force: bool) -> Result<(Vec<ResolvedDependency>, Vec<(String, String, String)>), String> {
        // Create cache directory
        if let Err(e) = std::fs::create_dir_all(&self.context.cache_dir) {
            return Err(format!("Error creating cache directory: {}", e));
        }

        let mut to_resolve = Vec::new();
        let mut version_changes = Vec::new();
        
        // Collect all dependencies (both runtime and dev)
        for (name, spec) in &package.dependencies {
            let dep_info = self.spec_to_dependency_info(name, spec)?;
            to_resolve.push(dep_info);
        }
        
        for (name, spec) in &package.dev_dependencies {
            let dep_info = self.spec_to_dependency_info(name, spec)?;
            to_resolve.push(dep_info);
        }

        // Resolve and install each dependency recursively
        for dep in to_resolve {
            if let Some(actual_version) = self.resolve_and_install_dependency_recursive(&dep, force)? {
                version_changes.push((dep.url.clone(), dep.version.clone(), actual_version));
            }
        }

        // Return the resolved dependencies and version changes
        Ok((self.context.resolved.values().cloned().collect(), version_changes))
    }

    fn spec_to_dependency_info(&self, name: &str, spec: &DependencySpec) -> Result<DependencyInfo, String> {
        let version = match spec {
            DependencySpec::Simple(v) => v.clone(),
            DependencySpec::Detailed { version, .. } => version.clone(),
        };

        // The name is actually the full URL (e.g., "github.com/user/repo")
        // Extract the actual package name from the URL
        let package_name = name.split('/').last().unwrap_or(name).to_string();

        Ok(DependencyInfo {
            name: package_name,
            url: name.to_string(), // name is the full URL
            version,
            is_local: false,
        })
    }

    fn resolve_and_install_dependency_recursive(&mut self, dep: &DependencyInfo, force: bool) -> Result<Option<String>, String> {
        // Check if already resolved
        if self.context.resolved.contains_key(&dep.name) {
            return Ok(None);
        }

        // Check for cycles
        if self.context.visiting.contains(&dep.name) {
            return Err(format!("Circular dependency detected: {}", dep.name));
        }

        self.context.visiting.insert(dep.name.clone());

        // Resolve and install the dependency
        let (resolved_version, version_changed) = self.install_dependency(&dep.url, &dep.version, force)?;
        
        // Create cache path
        let cache_path = self.context.cache_path_for(&dep.url, &resolved_version);

        // Load the dependency's alt.toml to find its dependencies
        let dep_alt_toml = cache_path.join("alt.toml");
        if dep_alt_toml.exists() {
            match Package::load_from_path(&dep_alt_toml) {
                Ok(dep_package) => {
                    // Recursively resolve sub-dependencies
                    for (sub_name, sub_spec) in &dep_package.dependencies {
                        let sub_dep_info = self.spec_to_dependency_info(sub_name, sub_spec)?;
                        self.resolve_and_install_dependency_recursive(&sub_dep_info, force)?;
                    }
                }
                Err(_) => {
                    // Ignore errors when parsing sub-dependencies
                }
            }
        }

        // Add to resolved
        let resolved = ResolvedDependency {
            name: dep.name.clone(),
            version: resolved_version.clone(),
        };

        self.context.resolved.insert(dep.name.clone(), resolved);
        self.context.visiting.remove(&dep.name);

        // Return the actual version if it changed
        if version_changed {
            Ok(Some(resolved_version))
        } else {
            Ok(None)
        }
    }

    fn install_dependency(&self, url: &str, version: &str, force: bool) -> Result<(String, bool), String> {
        println!("Fetching {}@{}...", url.split('/').last().unwrap_or(url), version);
        
        // Resolve version
        let resolved_version = if version == "*" || version == "latest" {
            "main".to_string()
        } else {
            version.to_string()
        };
        
        // Create cache path structure
        let sanitized_url = url.replace("://", "/");
        let repo_cache_dir = self.context.cache_dir.join(&sanitized_url);
        
        // Convert import URL to git URL
        let git_url = GitRepository::url_from_import_path(url);
        println!("  Cloning from: {}", git_url);
        
        // Create temporary clone location
        let temp_clone_dir = repo_cache_dir.join("_temp_clone");
        if temp_clone_dir.exists() {
            std::fs::remove_dir_all(&temp_clone_dir)
                .map_err(|e| format!("Failed to clean temp directory: {}", e))?;
        }
        
        // Clone the repository
        let git_repo = GitRepository::new(&git_url, temp_clone_dir.clone());
        git_repo.clone_if_needed()
            .map_err(|e| format!("Failed to clone repository: {}", e))?;
        
        // Try to checkout the specific version, get the actual version used
        let actual_version = match git_repo.checkout_version(&resolved_version) {
            Ok(()) => {
                // Always get the actual commit hash that was checked out
                // This ensures we have the full commit hash, even if input was a partial hash
                let current_commit = VersionResolver::get_current_commit_hash(&temp_clone_dir)
                    .unwrap_or_else(|_| resolved_version.clone());
                
                // For commit hashes, we want the full commit hash
                if is_commit_hash(&resolved_version) {
                    current_commit
                } else {
                    // For tags/branches, we can use either the tag name or the commit hash
                    // Let's use the commit hash for consistency
                    current_commit
                }
            }
            Err(e) => {
                println!("  Version '{}' not found ({}), falling back to 'main'", resolved_version, e);
                git_repo.checkout_version("main")
                    .map_err(|e| format!("Failed to checkout main branch: {}", e))?;
                
                // Get the actual commit hash of main
                VersionResolver::get_current_commit_hash(&temp_clone_dir)
                    .unwrap_or_else(|_| "main".to_string())
            }
        };
        
        // Now create the cache directory with the actual version
        let actual_version_cache_dir = repo_cache_dir.join(&actual_version);
        
        // Check if already cached with the actual version
        if actual_version_cache_dir.exists() && !force {
            if actual_version_cache_dir.join("alt.toml").exists() {
                println!("  Already cached at actual version, skipping (use --force to reinstall)");
                std::fs::remove_dir_all(&temp_clone_dir)
                    .map_err(|e| format!("Failed to clean temp directory: {}", e))?;
                return Ok((actual_version.clone(), version != &actual_version));
            }
        }
        
        // Verify this is a valid Althread package
        let alt_toml_path = temp_clone_dir.join("alt.toml");
        if !alt_toml_path.exists() {
            return Err(format!(
                "Invalid Althread package: {} does not contain alt.toml", 
                url
            ));
        }
        
        // Create the actual version cache directory
        std::fs::create_dir_all(&actual_version_cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        
        // Copy the entire repository contents to the versioned cache
        self.copy_dir_all(&temp_clone_dir, &actual_version_cache_dir)?;
        
        // Clean up temporary clone
        std::fs::remove_dir_all(&temp_clone_dir)
            .map_err(|e| format!("Failed to clean temp directory: {}", e))?;
        
        println!("  ✓ Cached to {}", actual_version_cache_dir.display());
        
        // Validate the package structure
        self.validate_package_structure(&actual_version_cache_dir)?;
        
        Ok((actual_version.clone(), version != &actual_version))
    }

    fn copy_dir_all(&self, src: &Path, dst: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dst)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
        
        for entry in std::fs::read_dir(src)
            .map_err(|e| format!("Failed to read directory: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let file_type = entry.file_type().map_err(|e| format!("Failed to get file type: {}", e))?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            
            // Skip .git directory and other hidden files
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with('.') {
                    continue;
                }
            }
            
            if file_type.is_dir() {
                self.copy_dir_all(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)
                    .map_err(|e| format!("Failed to copy file: {}", e))?;
            }
        }
        
        Ok(())
    }

    fn validate_package_structure(&self, package_dir: &Path) -> Result<(), String> {
        let alt_toml_path = package_dir.join("alt.toml");
        
        // Parse and validate alt.toml
        let package = Package::load_from_path(&alt_toml_path)
            .map_err(|e| format!("Failed to load alt.toml: {}", e))?;
        
        println!("  Package: {} v{}", package.package.name, package.package.version);
        if let Some(desc) = &package.package.description {
            println!("  Description: {}", desc);
        }
        
        // Look for .alt files
        let alt_files = self.find_alt_files(package_dir)?;
        if alt_files.is_empty() {
            println!("  Warning: No .alt files found in package");
        } else {
            println!("  Found {} .alt files", alt_files.len());
            for file in alt_files.iter().take(5) { // Show first 5
                let relative_path = file.strip_prefix(package_dir)
                    .unwrap_or(file)
                    .display();
                println!("    - {}", relative_path);
            }
            if alt_files.len() > 5 {
                println!("    ... and {} more", alt_files.len() - 5);
            }
        }
        
        Ok(())
    }

    fn find_alt_files(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        let mut alt_files = Vec::new();
        
        for entry in std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            
            if path.is_dir() {
                alt_files.extend(self.find_alt_files(&path)?);
            } else if let Some(extension) = path.extension() {
                if extension == "alt" {
                    alt_files.push(path);
                }
            }
        }
        
        Ok(alt_files)
    }

}

/// Helper function to check if a string looks like a commit hash
fn is_commit_hash(s: &str) -> bool {
    // A commit hash is typically 8 characters (short hash) or 40 characters (full hash)
    // and contains only hexadecimal characters
    (s.len() == 8 || s.len() == 40) && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_path_generation() {
        let context = ResolutionContext::new();
        let path = context.cache_path_for("github.com/user/repo", "v1.0.0");
        assert!(path.to_string_lossy().contains("github.com/user/repo/v1.0.0"));
    }

    #[test]
    fn test_dependency_resolution() {
        let resolver = DependencyResolver::new();
        
        // This test would need a mock package with dependencies
        // For now, just test that the resolver can be created
        assert!(resolver.context.resolved.is_empty());
    }

    #[test]
    fn test_parse_version_tag() {
        // Test various version tag formats
        assert!(VersionResolver::parse_version_tag("v1.0.0").is_ok());
        assert!(VersionResolver::parse_version_tag("V2.1.0").is_ok());
        assert!(VersionResolver::parse_version_tag("1.0.0").is_ok());
        
        // Test invalid version tags
        assert!(VersionResolver::parse_version_tag("invalid").is_err());
        assert!(VersionResolver::parse_version_tag("v1.0").is_err());
        assert!(VersionResolver::parse_version_tag("not-a-version").is_err());
    }

    #[test]
    fn test_version_parsing_edge_cases() {
        let version = VersionResolver::parse_version_tag("v1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);

        // Test with different prefixes
        assert!(VersionResolver::parse_version_tag("release1.0.0").is_ok());
        assert!(VersionResolver::parse_version_tag("version2.0.0").is_ok());
    }

    #[test]
    fn test_is_commit_hash() {
        // Test valid commit hashes
        assert!(is_commit_hash("a1b2c3d4")); // 8 character short hash
        assert!(is_commit_hash("1234567890abcdef1234567890abcdef12345678")); // 40 character full hash
        
        // Test invalid commit hashes
        assert!(!is_commit_hash("v1.0.0")); // version tag
        assert!(!is_commit_hash("main")); // branch name
        assert!(!is_commit_hash("latest")); // special version
        assert!(!is_commit_hash("a1b2c3")); // too short
        assert!(!is_commit_hash("a1b2c3d4e")); // wrong length
        assert!(!is_commit_hash("z1b2c3d4")); // non-hex character
    }
}
