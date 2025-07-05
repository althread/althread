use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub package: PackageInfo,
    #[serde(default)]
    pub dependencies: std::collections::HashMap<String, DependencySpec>,
    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: std::collections::HashMap<String, DependencySpec>,
    #[serde(default)]
    pub workspace: Option<WorkspaceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    Simple(String),
    Detailed {
        version: String,
        #[serde(default)]
        features: Vec<String>,
        #[serde(default)]
        optional: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub members: Vec<String>,
}

impl Package {
    pub fn new(name: String, version: String) -> Self {
        Self {
            package: PackageInfo {
                name,
                version,
                description: None,
                authors: None,
                license: None,
            },
            dependencies: std::collections::HashMap::new(),
            dev_dependencies: std::collections::HashMap::new(),
            workspace: None,
        }
    }

    pub fn load_from_path(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let package: Package = toml::from_str(&contents)?;
        Ok(package)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let contents = toml::to_string_pretty(&self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    pub fn add_dependency(&mut self, name: String, spec: DependencySpec) {
        self.dependencies.insert(name, spec);
    }

    pub fn add_dev_dependency(&mut self, name: String, spec: DependencySpec) {
        self.dev_dependencies.insert(name, spec);
    }

    pub fn remove_dependency(&mut self, name: &str) -> bool {
        self.dependencies.remove(name).is_some() || self.dev_dependencies.remove(name).is_some()
    }
}

#[derive(Debug, Clone)]
pub struct DependencyInfo {
    pub name: String,
    pub url: String,
    pub version: String,
    pub is_local: bool,
}

impl DependencyInfo {
    pub fn parse(dep_string: &str) -> Result<Self, String> {
        // Parse dependency strings like:
        // - "utils/math" (local)
        // - "github.com/username/repo" (remote, latest)
        // - "github.com/username/repo@v1.0.0" (remote, specific version)
        // - "github.com/username/repo@main" (remote, specific branch)
        
        if dep_string.contains("://") {
            return Err("URL schemes not supported, use domain-based imports".to_string());
        }

        let (url, version) = if let Some(at_pos) = dep_string.find('@') {
            let url = dep_string[..at_pos].to_string();
            let version = dep_string[at_pos + 1..].to_string();
            (url, version)
        } else {
            (dep_string.to_string(), "latest".to_string())
        };

        let is_local = !url.contains('.') || url.starts_with("./") || url.starts_with("../");
        
        // For remote dependencies, the name should be the last part of the URL
        // For local dependencies, the name should be the last part of the path
        let name = if is_local {
            url.split('/').last().unwrap_or(&url).to_string()
        } else {
            url.split('/').last().unwrap_or(&url).to_string()
        };

        Ok(DependencyInfo {
            name,
            url,
            version,
            is_local,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_local_dependency() {
        let dep = DependencyInfo::parse("utils/math").unwrap();
        assert_eq!(dep.name, "math");
        assert_eq!(dep.url, "utils/math");
        assert_eq!(dep.version, "latest");
        assert!(dep.is_local);
    }

    #[test]
    fn test_parse_remote_dependency() {
        let dep = DependencyInfo::parse("github.com/username/repo@v1.0.0").unwrap();
        assert_eq!(dep.name, "repo");
        assert_eq!(dep.url, "github.com/username/repo");
        assert_eq!(dep.version, "v1.0.0");
        assert!(!dep.is_local);
    }

    #[test]
    fn test_parse_remote_dependency_latest() {
        let dep = DependencyInfo::parse("github.com/username/repo").unwrap();
        assert_eq!(dep.name, "repo");
        assert_eq!(dep.url, "github.com/username/repo");
        assert_eq!(dep.version, "latest");
        assert!(!dep.is_local);
    }
}
