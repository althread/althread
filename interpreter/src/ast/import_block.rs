use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::identifier::Identifier,
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::AlthreadResult,
    no_rule,
    parser::Rule,
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
}

impl NodeBuilder for ImportBlock {
    fn build(pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let mut imports = Vec::new();
        
        for pair in pairs {
            match pair.as_rule() {
                Rule::import_list => {
                    for import_item_pair in pair.into_inner() {
                        let import_item = Node::build(import_item_pair)?;
                        imports.push(import_item);
                    }
                }
                _ => return Err(no_rule!(pair, "ImportBlock")),
            }
        }

        Ok(Self { imports })
    }
}

impl NodeBuilder for ImportItem {
    fn build(pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let mut pairs = pairs;
        let path_pair = pairs.next().unwrap();
        let path = ImportPath::build(path_pair.into_inner())?;
        
        let alias = if let Some(next_pair) = pairs.next() {
            match next_pair.as_rule() {
                Rule::AS_KW => {
                    // The identifier should be the next pair after "as"
                    if let Some(identifier_pair) = pairs.next() {
                        Some(Node::build(identifier_pair)?)
                    } else {
                        return Err(no_rule!(next_pair, "ImportItem - missing identifier after 'as'"));
                    }
                }
                Rule::identifier => {
                    // If we get an identifier directly
                    Some(Node::build(next_pair)?)
                }
                _ => return Err(no_rule!(next_pair, "ImportItem")),
            }
        } else {
            None
        };

        Ok(Self { path, alias })
    }
}

impl ImportPath {
    fn build(pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let mut segments = Vec::new();
        
        for pair in pairs {
            match pair.as_rule() {
                Rule::identifier => {
                    segments.push(pair.as_str().to_string());
                }
                _ => return Err(no_rule!(pair, "ImportPath")),
            }
        }

        Ok(Self { segments })
    }
}

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
            writeln!(f, "{prefix}import {} as {}", self.path.to_string(), alias.value.value)?;
        } else {
            writeln!(f, "{prefix}import {}", self.path.to_string())?;
        }
        Ok(())
    }
}