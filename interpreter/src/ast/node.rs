use std::{collections::HashMap, fmt, hash::Hash};

use pest::iterators::{Pair, Pairs};

use crate::{compiler::State, env::{instruction::Instruction, process_env::ProcessEnv, symbol_table::symbol::Symbol}, error::AlthreadResult, parser::Rule};
use crate::env::instruction::ProcessCode;

use super::{
    display::{AstDisplay, Prefix}, statement::expression::{primary_expression::PrimaryExpression, Expression}, token::literal::Literal
};

#[derive(Debug, Clone)]
pub struct Node<T> {
    pub value: T,
    pub line: usize,
    pub column: usize,
}

pub trait NodeBuilder: Sized {
    fn build(pairs: Pairs<Rule>) -> AlthreadResult<Self>;
}

pub trait NodeExecutor: Sized {
    fn eval(&self, env: &mut ProcessEnv) -> AlthreadResult<Option<Literal>>;
}

pub trait NodeExecutor2: Sized {
    fn exec(&self, env: &mut ProcessEnv) -> AlthreadResult<()>;
}

pub trait InstructionBuilder: Sized {
    fn compile(&self, state: &mut State) -> Vec<Instruction>;
}


impl<T: NodeBuilder> Node<T> {
    pub fn build(pair: Pair<Rule>) -> AlthreadResult<Self> {
        let (line, col) = pair.line_col();
        Ok(Node {
            value: T::build(pair.into_inner())?,
            line,
            column: col,
        })
    }
}

impl<T: NodeExecutor> Node<T> {
    pub fn eval(&self, env: &mut ProcessEnv) -> AlthreadResult<Option<Literal>> {
        self.value.eval(env)
    }
}

impl<T: AstDisplay> AstDisplay for Node<T> {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        self.value.ast_fmt(f, prefix)
    }
}

impl<T: fmt::Display> fmt::Display for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T: InstructionBuilder> Node<T> {
    pub fn compile(&self, state: &mut State) -> Vec<Instruction> {
        self.value.compile(state)
    }
}

/*
impl Node<Expression> {
    pub fn compile(&self, state: &mut State) -> Vec<Instruction> {
        let mut local_ast = self.clone();
        let mut global_read: HashMap<&String,usize>  = HashMap::new(); 

        // retrieve first the list of used global variables

        //Then replace the global variables and local variables with their respective indexes in the stack

        match self.value {
            Expression::Primary(node) => {
                match node.value {
                    PrimaryExpression::Identifier(ident) => {
                        if let Some(index) = state.global_table.get(ident.value.value.as_str()) {
                            let index = global_read.get(&ident.value.value).or_insert_with(|| {
                                state.global_table.len()
                            });
                            local_ast.node = PrimaryExpression::LocalRead(Node {
                                value: LocalRead {
                                    index: index,
                                },
                                line: 0,
                                column: 0,
                            });
                        } else {
                            let mut var_idx = 0;
                            for var in state.program_stack.iter().rev() {
                                if var.name = ident.value.value {
                                    local_ast.node = PrimaryExpression::LocalRead(Node {
                                        value: LocalRead {
                                            index: var_idx,
                                            //TODO: add the number of used global variables in the index of the local variables
                                        },
                                        line: 0,
                                        column: 0,
                                    });
                                }
                                var_idx += 1;
                            }
                        }
                    }
                    _ => {}
                }
            },
            Expression::Binary(node) => {
                node.left.compile(state);
                node.right.compile(state);
            },
            Expression::Unary(node) => {
                node.right.compile(state);
            },
        }
        // Then add the global variables to the stack (and instruction to add the value of the global variables)
        // add the instruction to run the local AST
        // unstack the global variables

        self.get_global_read(self.value);
        self.value.compile(process_code, env)
    }
}
    */