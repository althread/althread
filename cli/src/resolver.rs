use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use crate::package::{Package, DependencyInfo, DependencySpec};

#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    pub name: String,
    pub url: String,
    pub version: String,
    pub cache_path: PathBuf,
    pub dependencies: Vec<String>, // Names of dependencies this depends on
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

    pub fn resolve_dependencies(&mut self, package: &Package) -> Result<Vec<ResolvedDependency>, String> {
        let mut to_resolve = Vec::new();
        
        // Collect all dependencies (both runtime and dev)
        for (name, spec) in &package.dependencies {
            let dep_info = self.spec_to_dependency_info(name, spec)?;
            to_resolve.push(dep_info);
        }
        
        for (name, spec) in &package.dev_dependencies {
            let dep_info = self.spec_to_dependency_info(name, spec)?;
            to_resolve.push(dep_info);
        }

        // Resolve each dependency recursively
        for dep in to_resolve {
            self.resolve_dependency_recursive(&dep)?;
        }

        // Return the resolved dependencies in dependency order
        Ok(self.context.resolved.values().cloned().collect())
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

    fn resolve_dependency_recursive(&mut self, dep: &DependencyInfo) -> Result<(), String> {
        // Check if already resolved
        if self.context.resolved.contains_key(&dep.name) {
            return Ok(());
        }

        // Check for cycles
        if self.context.visiting.contains(&dep.name) {
            return Err(format!("Circular dependency detected: {}", dep.name));
        }

        self.context.visiting.insert(dep.name.clone());

        // Resolve the actual version (for now, just use the specified version)
        let resolved_version = self.resolve_version(&dep.url, &dep.version)?;
        
        // Create cache path
        let cache_path = self.context.cache_path_for(&dep.url, &resolved_version);

        // Check if already cached
        if !cache_path.exists() {
            // For now, we'll let the install command handle the actual fetching
            // Just record that we need to fetch this dependency
            println!("  Need to fetch: {}@{}", dep.name, resolved_version);
        }

        // Try to load the dependency's alt.toml to find its dependencies
        let dep_alt_toml = cache_path.join("alt.toml");
        let sub_dependencies = if dep_alt_toml.exists() {
            match Package::load_from_path(&dep_alt_toml) {
                Ok(dep_package) => {
                    let mut sub_deps = Vec::new();
                    
                    // Recursively resolve sub-dependencies
                    for (sub_name, sub_spec) in &dep_package.dependencies {
                        let sub_dep_info = self.spec_to_dependency_info(sub_name, sub_spec)?;
                        self.resolve_dependency_recursive(&sub_dep_info)?;
                        sub_deps.push(sub_name.clone());
                    }
                    
                    sub_deps
                }
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        // Add to resolved
        let resolved = ResolvedDependency {
            name: dep.name.clone(),
            url: dep.url.clone(),
            version: resolved_version,
            cache_path,
            dependencies: sub_dependencies,
        };

        self.context.resolved.insert(dep.name.clone(), resolved);
        self.context.visiting.remove(&dep.name);

        Ok(())
    }

    fn resolve_version(&self, _url: &str, version: &str) -> Result<String, String> {
        // For now, just return the version as-is
        // In a real implementation, we'd:
        // 1. Fetch available versions from the repository
        // 2. Apply semantic versioning rules
        // 3. Find the best matching version
        
        if version == "*" || version == "latest" {
            // For now, assume we want "main" or "master" branch
            Ok("main".to_string())
        } else {
            Ok(version.to_string())
        }
    }
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
        let mut resolver = DependencyResolver::new();
        
        // This test would need a mock package with dependencies
        // For now, just test that the resolver can be created
        assert!(resolver.context.resolved.is_empty());
    }
}
