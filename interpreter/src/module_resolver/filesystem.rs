use crate::error::{AlthreadError, AlthreadResult, ErrorType, Pos};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub trait FileSystem {
    fn is_file(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn canonicalize(&self, path: &Path) -> AlthreadResult<PathBuf>;
    fn read_file(&self, path: &Path) -> AlthreadResult<String>;
}

pub struct StandardFileSystem;

impl Clone for StandardFileSystem {
    fn clone(&self) -> Self {
        StandardFileSystem
    }
}

impl FileSystem for StandardFileSystem {
    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn canonicalize(&self, path: &Path) -> AlthreadResult<PathBuf> {
        std::fs::canonicalize(path).map_err(|e| {
            crate::error::AlthreadError::new(
                ErrorType::RuntimeError,
                Some(Pos::default()),
                format!("Failed to resolve path: {}", e),
            )
        })
    }
    fn read_file(&self, path: &Path) -> AlthreadResult<String> {
        std::fs::read_to_string(path).map_err(|e| {
            AlthreadError::new(
                ErrorType::ModuleNotFound,
                Some(Pos::default()),
                format!("Failed to read file: {}", e),
            )
        })
    }
}

pub struct VirtualFileSystem {
    files: HashMap<String, String>,
}

impl Clone for VirtualFileSystem {
    fn clone(&self) -> Self {
        VirtualFileSystem {
            files: self.files.clone(),
        }
    }
}

impl VirtualFileSystem {
    pub fn new(files: HashMap<String, String>) -> Self {
        Self { files }
    }
}

impl FileSystem for VirtualFileSystem {
    fn is_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_string();

        // Try the path as-is first
        if self.files.contains_key(&path_str) {
            return true;
        }

        // Try without leading "./" if present
        let cleaned_path = if path_str.starts_with("./") {
            &path_str[2..]
        } else {
            &path_str
        };

        self.files.contains_key(cleaned_path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_string();
        let prefix = if path_str.ends_with('/') {
            path_str.clone()
        } else {
            format!("{}/", path_str)
        };

        // Check if any file starts with this path as a directory
        self.files
            .keys()
            .any(|file_path| file_path.starts_with(&prefix))
    }

    fn canonicalize(&self, path: &Path) -> AlthreadResult<PathBuf> {
        let path_str = path.to_string_lossy().to_string();

        // Try the path as-is first
        if self.files.contains_key(&path_str) {
            return Ok(path.to_path_buf());
        }

        // Try without leading "./" if present
        let cleaned_path = if path_str.starts_with("./") {
            &path_str[2..]
        } else {
            &path_str
        };

        if self.files.contains_key(cleaned_path) {
            return Ok(PathBuf::from(cleaned_path));
        }

        Err(AlthreadError::new(
            ErrorType::RuntimeError,
            Some(Pos::default()),
            format!("File not found in virtual filesystem: {}", path_str),
        ))
    }

    fn read_file(&self, path: &Path) -> AlthreadResult<String> {
        let path_str = path.to_string_lossy().to_string();

        // Try the path as-is first
        if let Some(content) = self.files.get(&path_str) {
            return Ok(content.clone());
        }

        // Try without leading "./" if present
        let cleaned_path = if path_str.starts_with("./") {
            &path_str[2..]
        } else {
            &path_str
        };

        self.files.get(cleaned_path).cloned().ok_or_else(|| {
            AlthreadError::new(
                ErrorType::ModuleNotFound,
                Some(Pos::default()),
                format!("File not found in virtual filesystem: {}", path_str),
            )
        })
    }
}
