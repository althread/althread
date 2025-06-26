use std::{collections::HashMap, fs, path::{Path, PathBuf}};

use crate::{ast::import_block::{ImportBlock, ImportItem, ImportPath}, error::{AlthreadError, AlthreadResult, ErrorType, Pos}};


#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub name: String,
    pub path: PathBuf,
    pub alias: Option<String>,
}

#[derive(Debug)]
pub struct ModuleResolver {
    pub current_file_dir: PathBuf,
    pub resolved_modules: HashMap<String, ResolvedModule>
}

impl ModuleResolver {
    pub fn new(current_file: &Path) -> Self {
        let current_file_dir = current_file.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        Self {
            current_file_dir,
            resolved_modules: HashMap::new(),
        }
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
        let module_path = self.resolve_path(&item.path)?;
        let name = item.path.last_segment().to_string();
        let alias = item.alias.as_ref().map(|alias| alias.value.value.clone());

        Ok(ResolvedModule {
            name, 
            path: module_path,
            alias
        })
    }

    fn resolve_path(&self, import_path: &ImportPath) -> AlthreadResult<PathBuf> {
        let relative_path_str = import_path.segments.join("/");
        let mut path = self.current_file_dir.clone();
        path.push(&relative_path_str);

        let path_with_extension = path.with_extension("alt");

        if path_with_extension.is_file() {
            return Ok(fs::canonicalize(path_with_extension)
                .map_err(|e| AlthreadError::new(
                    ErrorType::RuntimeError,
                    Some(Pos::default()),
                    format!("Failed to resolve module path: {}", e)
                ))?);
        }


        Err(AlthreadError::new(
            ErrorType::ModuleNotFound,
            Some(Pos::default()),
            format!("Module '{}' not found in path '{}'", import_path.to_string(), self.current_file_dir.display())
        ))
    }
}