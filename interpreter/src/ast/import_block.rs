use std::{collections::HashSet, fmt};

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node},
        token::identifier::Identifier,
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::{AlthreadError, AlthreadResult, ErrorType},
};

#[derive(Debug, Clone)]
pub struct ImportBlock {
    pub imports: Vec<Node<ImportItem>>,
}

#[derive(Debug, Clone)]
pub struct ImportItem {
    pub path: ImportPath,
    pub alias: Option<Node<Identifier>>,
}

#[derive(Debug, Clone)]
pub struct ImportPath {
    pub segments: Vec<String>,
}

impl ImportPath {
    pub fn to_string(&self) -> String {
        self.segments.join("/")
    }

    pub fn last_segment(&self) -> &str {
        self.segments.last().map(|s| s.as_str()).unwrap_or("")
    }
}

impl ImportBlock {
    pub(crate) fn validate_import_names(imports: &[Node<ImportItem>]) -> AlthreadResult<()> {
        let mut used_names = HashSet::new();

        for import in imports {
            let import_name = if let Some(alias) = &import.value.alias {
                alias.value.value.clone()
            } else {
                import.value.path.last_segment().to_string()
            };

            if used_names.contains(&import_name) {
                return Err(AlthreadError::new(
                    ErrorType::ImportNameConflict,
                    Some(import.pos.clone()),
                    format!(
                        "'{}' is already imported. Use 'as' to provide a unique alias.",
                        import_name,
                    ),
                ));
            }

            used_names.insert(import_name);
        }
        Ok(())
    }
}

impl ImportPath {}

impl InstructionBuilder for ImportBlock {
    fn compile(&self, _state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        // Import blocks don't generate runtime instructions
        // Module resolution happens at compile time
        Ok(InstructionBuilderOk::new())
    }
}

impl AstDisplay for ImportBlock {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}import_block")?;

        let mut import_count = self.imports.len();
        for import in &self.imports {
            import_count -= 1;
            if import_count == 0 {
                import.ast_fmt(f, &prefix.add_leaf())?;
            } else {
                import.ast_fmt(f, &prefix.add_branch())?;
            }
        }

        Ok(())
    }
}

impl AstDisplay for ImportItem {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        if let Some(alias) = &self.alias {
            writeln!(
                f,
                "{prefix}import {} as {}",
                self.path.to_string(),
                alias.value.value
            )?;
        } else {
            writeln!(f, "{prefix}import {}", self.path.to_string())?;
        }
        Ok(())
    }
}
