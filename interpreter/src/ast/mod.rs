pub mod block;
pub mod condition_block;
pub mod import_block;
pub mod display;
pub mod node;
pub mod statement;
pub mod token;


use std::{
    collections::HashMap, fmt::{self, Formatter}
};

use block::Block;
use condition_block::ConditionBlock;
use import_block::ImportBlock;
use display::{AstDisplay, Prefix};
use node::{Node};
use pest::{iterators::Pairs};
use token::{args_list::ArgsList, condition_keyword::ConditionKeyword, datatype::DataType};

use crate::{error::{AlthreadError, AlthreadResult, ErrorType, Pos}, no_rule, parser::Rule};


#[derive(Debug)]
pub struct Ast {
    pub process_blocks: HashMap<String, (Node<ArgsList>, Node<Block>)>,
    pub condition_blocks: HashMap<ConditionKeyword, Node<ConditionBlock>>,
    pub global_block: Option<Node<Block>>,
    pub function_blocks: HashMap<String, (Node<ArgsList>, DataType, Node<Block>, bool)>,
    pub import_block: Option<Node<ImportBlock>>
}

impl Ast {
    pub fn new() -> Self {
        Self {
            process_blocks: HashMap::new(),
            condition_blocks: HashMap::new(),
            global_block: None,
            function_blocks: HashMap::new(),
            import_block: None,
        }
    }
    /// Builds an AST from the given pairs of rules.
    pub fn build(pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let mut ast = Self::new();
        for pair in pairs {
            match pair.as_rule() {
                Rule::import_block => {
                    if ast.import_block.is_some() {
                        return Err(AlthreadError::new(
                            ErrorType::SyntaxError,
                            Some(Pos::from_span(pair.as_span(), filepath)),
                            "Only one import block is allowed per file.".to_string(),
                        ));
                    }

                    let import_block = Node::build(pair, filepath)?;
                    ast.import_block = Some(import_block);
                }
                Rule::main_block => {
                    let mut pairs = pair.into_inner();

                    let main_block = Node::build(pairs.next().unwrap(), filepath)?;
                    ast.process_blocks
                        .insert("main".to_string(), (Node::<ArgsList>::new(), main_block));
                }
                Rule::global_block => {
                    let mut pairs = pair.into_inner();

                    let global_block = Node::build(pairs.next().unwrap(), filepath)?;
                    ast.global_block = Some(global_block);
                }
                Rule::condition_block => {
                    let mut pairs = pair.into_inner();

                    let keyword_pair = pairs.next().unwrap();
                    let condition_keyword = match keyword_pair.as_rule() {
                        Rule::ALWAYS_KW => ConditionKeyword::Always,
                        Rule::NEVER_KW => ConditionKeyword::Never,
                        Rule::EVENTUALLY_KW => ConditionKeyword::Eventually,
                        _ => return Err(no_rule!(keyword_pair, "condition keyword", filepath)),
                    };
                    let condition_block = Node::build(pairs.next().unwrap(), filepath)?;
                    ast.condition_blocks
                        .insert(condition_keyword, condition_block);
                }
                Rule::program_block => {
                    let mut pairs = pair.into_inner();

                    let process_identifier = pairs.next().unwrap().as_str().to_string();
                    let args_list: Node<token::args_list::ArgsList> =
                        Node::build(pairs.next().unwrap(), filepath)?;
                    let program_block = Node::build(pairs.next().unwrap(), filepath)?;
                    ast.process_blocks
                        .insert(process_identifier, (args_list, program_block));
                }
                Rule::function_block => {
                    let mut pairs  = pair.into_inner();

                    // check for directive
                    let mut is_private = false;
                    let first = pairs.peek().unwrap();
                    if first.as_rule() == Rule::private_directive {
                        is_private = true;
                        pairs.next(); // consume the private directive
                    }

                    let function_identifier = pairs.next().unwrap().as_str().to_string();

                    let args_list: Node<token::args_list::ArgsList> = Node::build(pairs.next().unwrap(), filepath)?;
                    pairs.next(); // skip the "->" token
                    let return_datatype = DataType::from_str(pairs.next().unwrap().as_str());

                    let function_block: Node<Block>  = Node::build(pairs.next().unwrap(), filepath)?;

                    // check if function definition is already defined
                    if ast.function_blocks.contains_key(&function_identifier) {
                        return Err(AlthreadError::new(
                            ErrorType::FunctionAlreadyDefined,
                            Some(function_block.pos),
                            format!("Function '{}' is already defined", function_identifier),
                        ));
                    }

                    ast.function_blocks
                        .insert(
                        function_identifier,
                        (args_list, return_datatype, function_block, is_private)
                    );

                }
                Rule::EOI => (),
                _ => return Err(no_rule!(pair, "root ast", filepath)),
            }
        }

        Ok(ast)
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
        }

        for (process_name, (_args, process_node)) in &self.process_blocks {
            writeln!(f, "{}{}", prefix, process_name)?;
            process_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
        }

        for (function_name, (_args, return_type, function_node, is_private)) in &self.function_blocks {
            writeln!(f, "{}", if *is_private { "@private " } else { "" })?;
            writeln!(f, "{}{} -> {}", prefix, function_name, return_type)?;
            function_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
        }

        Ok(())
    }
}
