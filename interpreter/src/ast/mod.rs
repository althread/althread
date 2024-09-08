pub mod block;
pub mod display;
pub mod node;
pub mod statement;
pub mod token;


use core::panic;
use std::{
    collections::HashMap,
    fmt::{self, Formatter},
};

use block::Block;
use display::{AstDisplay, Prefix};
use node::Node;
use pest::iterators::Pairs;
use statement::{expression::Expression, Statement};
use token::{condition_keyword::ConditionKeyword, literal::Literal};

use crate::{
    compiler::{CompiledProject, CompilerState}, error::{AlthreadError, AlthreadResult, ErrorType}, no_rule, parser::Rule, vm::{instruction::{Instruction, InstructionType, ProgramCode}, VM}
};

#[derive(Debug)]
pub struct Ast {
    pub process_blocks: HashMap<String, Node<Block>>,
    pub condition_blocks: HashMap<ConditionKeyword, Node<Block>>,
    pub global_block: Option<Node<Block>>,
}

impl Ast {
    pub fn new() -> Self {
        Self {
            process_blocks: HashMap::new(),
            condition_blocks: HashMap::new(),
            global_block: None,
        }
    }

    pub fn build(pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let mut ast = Self::new();
        for pair in pairs {
            match pair.as_rule() {
                Rule::main_block => {
                    let mut pairs = pair.into_inner();

                    let main_block = Node::build(pairs.next().unwrap())?;
                    ast.process_blocks.insert("main".to_string(), main_block);
                }
                Rule::global_block => {
                    let mut pairs = pair.into_inner();

                    let global_block = Node::build(pairs.next().unwrap())?;
                    ast.global_block = Some(global_block);
                }
                Rule::condition_block => {
                    let mut pairs = pair.into_inner();

                    let keyword_pair = pairs.next().unwrap();
                    let condition_keyword = match keyword_pair.as_rule() {
                        Rule::ALWAYS_KW => ConditionKeyword::Always,
                        Rule::NEVER_KW => ConditionKeyword::Never,
                        _ => return Err(no_rule!(keyword_pair)),
                    };
                    let condition_block = Node::build(pairs.next().unwrap())?;
                    ast.condition_blocks
                        .insert(condition_keyword, condition_block);
                }
                Rule::process_block => {
                    let mut pairs = pair.into_inner();

                    let process_identifier = pairs.next().unwrap().as_str().to_string();
                    let process_block = Node::build(pairs.next().unwrap())?;
                    ast.process_blocks.insert(process_identifier, process_block);
                }
                Rule::EOI => (),
                _ => return Err(no_rule!(pair)),
            }
        }

        Ok(ast)
    }

    pub fn compile(&self) -> AlthreadResult<CompiledProject> {

        // "compile" the "shared" block to retrieve the set of 
        // shared variables
        let mut state = CompilerState::new();
        let mut global_memory = HashMap::new();
        let mut global_table = HashMap::new();
        state.current_stack_depth = 1;
        let memory = match self.global_block.as_ref() {
            Some(global) => {
                let mut memory = VM::new_memory();
                for node in global.value.children.iter() {
                    match &node.value {
                        Statement::Declaration(decl) => {
                            let mut literal = None;
                            for gi in node.compile(&mut state)? {
                                match gi.control {
                                    InstructionType::Expression(exp) => {
                                        literal = Some(exp.root.eval(&memory).or_else(|err| Err(AlthreadError::new(
                                            ErrorType::ExpressionError, 
                                            gi.line,
                                            gi.column,
                                            err
                                            )))?);
                                    },
                                    InstructionType::Declaration(dec) => {
                                        // do nothing
                                        assert!(dec.unstack_len == 1)
                                    }
                                    InstructionType::PushNull(datatype) => {
                                        memory.push(datatype.default())
                                    }
                                    _ => {
                                        panic!("unexpected instruction in compiled declaration statement")
                                    }
                                }
                            }
                            let literal = literal.expect("declaration did not compiled to expression nor PushNull");
                            memory.push(literal);

                            let var_name = &decl.value.identifier.value.value;
                            global_table.insert(var_name.clone(), state.program_stack.last().unwrap().clone());
                            global_memory.insert(var_name.clone(), memory.last().unwrap().clone());
                        },
                        _ => return Err(AlthreadError::new(
                            ErrorType::InstructionNotAllowed, 
                            node.line,
                            node.column,
                            "The 'shared' block can only contains assignment from an expression".to_string()
                            )),
                    }
                    
                }
                memory
            }
            None => Vec::new()
        };

        state.global_table = global_table;

        state.unstack_current_depth();
        assert!(state.current_stack_depth == 0);

        let mut programs_code = HashMap::new();
        for (name, _) in self.process_blocks.iter() {
            let code = self.compile_program(name, &mut state)?;
            programs_code.insert(name.clone(), code);
            assert!(state.current_stack_depth == 0);
        }

        Ok(CompiledProject {
            global_memory,
            programs_code
        })
    }
    fn compile_program(&self, name: &str, state: &mut CompilerState) -> AlthreadResult<ProgramCode> {

        let mut process_code = ProgramCode {
            instructions: Vec::new(),
            name: name.to_string(),
        };
        let prog = self.process_blocks.get(name).expect("trying to compile a non-existant program");
        process_code.instructions = prog.compile(state)?;
        process_code.instructions.push(Instruction {
            control: InstructionType::EndProgram,
            line: prog.line,
            column: prog.column,
        });
        Ok(process_code)
    }

}

impl fmt::Display for Ast {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.ast_fmt(f, &Prefix::new())
    }
}

impl AstDisplay for Ast {
    fn ast_fmt(&self, f: &mut Formatter, prefix: &Prefix) -> fmt::Result {
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

        for (process_name, process_node) in &self.process_blocks {
            writeln!(f, "{}{}", prefix, process_name)?;
            process_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
        }

        Ok(())
    }
}
