pub mod block;
pub mod condition_block;
pub mod display;
pub mod import_block;
pub mod node;
pub mod statement;
pub mod token;

use std::{
    collections::HashMap,
    fmt::{self, Formatter},
};

use block::Block;
use condition_block::ConditionBlock;
use display::{AstDisplay, Prefix};
use import_block::ImportBlock;
use node::Node;
use token::{args_list::ArgsList, condition_keyword::ConditionKeyword, datatype::DataType};

use crate::checker::ltl::ast::CheckBlock;

#[derive(Debug)]
pub struct Ast {
    pub process_blocks: HashMap<String, (Node<ArgsList>, Node<Block>, bool)>,
    pub condition_blocks: HashMap<ConditionKeyword, Node<ConditionBlock>>,
    pub check_blocks: Vec<Node<CheckBlock>>,
    pub global_block: Option<Node<Block>>,
    pub function_blocks: HashMap<String, (Node<ArgsList>, DataType, Node<Block>, bool)>,
    pub import_block: Option<Node<ImportBlock>>,
}

impl Ast {
    pub fn new() -> Self {
        Self {
            process_blocks: HashMap::new(),
            condition_blocks: HashMap::new(),
            check_blocks: Vec::new(),
            global_block: None,
            function_blocks: HashMap::new(),
            import_block: None,
        }
    }

    pub fn diff_summary(&self, other: &Self) -> Option<String> {
        let lhs = self.canonical_repr();
        let rhs = other.canonical_repr();
        if lhs == rhs {
            return None;
        }

        let mut lhs_lines = lhs.lines();
        let mut rhs_lines = rhs.lines();
        for line_number in 1.. {
            match (lhs_lines.next(), rhs_lines.next()) {
                (Some(left), Some(right)) if left == right => continue,
                (Some(left), Some(right)) => {
                    return Some(format!(
                        "AST mismatch at line {line_number}: expected `{left}`, got `{right}`"
                    ));
                }
                (Some(left), None) => {
                    return Some(format!(
                        "AST mismatch at line {line_number}: unexpected extra line `{left}`"
                    ));
                }
                (None, Some(right)) => {
                    return Some(format!(
                        "AST mismatch at line {line_number}: missing line, got `{right}`"
                    ));
                }
                (None, None) => break,
            }
        }

        Some("AST mismatch".to_string())
    }

    fn canonical_repr(&self) -> String {
        let mut out = String::new();
        if let Some(import_block) = &self.import_block {
            out.push_str("import\n");
            out.push_str(&format!("{import_block:?}\n"));
        }
        if let Some(global_block) = &self.global_block {
            out.push_str("shared\n");
            out.push_str(&format!("{global_block:?}\n"));
        }

        let mut condition_entries = self.condition_blocks.iter().collect::<Vec<_>>();
        condition_entries.sort_by_key(|(keyword, _)| format!("{keyword:?}"));
        for (keyword, block) in condition_entries {
            out.push_str(&format!("condition:{keyword:?}\n{block:?}\n"));
        }

        for check_block in &self.check_blocks {
            out.push_str(&format!("check:{check_block:?}\n"));
        }

        let mut process_entries = self.process_blocks.iter().collect::<Vec<_>>();
        process_entries.sort_by_key(|(name, _)| (*name).clone());
        for (name, value) in process_entries {
            out.push_str(&format!("process:{name}:{value:?}\n"));
        }

        let mut function_entries = self.function_blocks.iter().collect::<Vec<_>>();
        function_entries.sort_by_key(|(name, _)| (*name).clone());
        for (name, value) in function_entries {
            out.push_str(&format!("function:{name}:{value:?}\n"));
        }

        out
    }
}

impl fmt::Display for Ast {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.ast_fmt(f, &Prefix::new())
    }
}

impl AstDisplay for Ast {
    fn ast_fmt(&self, f: &mut Formatter, prefix: &Prefix) -> fmt::Result {
        if let Some(import_block) = &self.import_block {
            import_block.ast_fmt(f, prefix)?;
            writeln!(f, "")?;
        }

        if let Some(global_node) = &self.global_block {
            writeln!(f, "{}shared", prefix)?;
            global_node.ast_fmt(f, &prefix.add_branch())?;
        }

        writeln!(f, "")?;

        for (condition_name, condition_node) in &self.condition_blocks {
            writeln!(f, "{}{}", prefix, condition_name)?;
            condition_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
            for check_block in &self.check_blocks {
                writeln!(f, "{}check", prefix)?;
                for form in &check_block.value.formulas {
                    writeln!(f, "{}{}", prefix.add_branch(), form)?;
                }
            }
        }

        for (process_name, (_args, process_node, is_private)) in &self.process_blocks {
            let process_name = if *is_private {
                format!("@private {}", process_name)
            } else {
                process_name.clone()
            };
            writeln!(f, "{}{}", prefix, process_name)?;
            process_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
        }

        for (function_name, (_args, return_type, function_node, is_private)) in
            &self.function_blocks
        {
            writeln!(f, "{}", if *is_private { "@private " } else { "" })?;
            writeln!(f, "{}{} -> {}", prefix, function_name, return_type)?;
            function_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
        }

        Ok(())
    }
}
