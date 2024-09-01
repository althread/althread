use std::collections::HashMap;

use crate::ast::token::datatype::DataType;

#[derive(Debug, Clone)]
pub struct Variable {
    pub mutable: bool,
    pub name: String,
    pub datatype: DataType,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct State {
    pub global_table: HashMap<String, Variable>,
    pub program_stack: Vec<Variable>,
    pub current_stack_depth: usize,
    pub is_atomic: bool,
}

impl State {
    pub fn new() -> Self {
        Self {
            global_table: HashMap::new(),
            program_stack: Vec::new(),
            current_stack_depth: 0,
            is_atomic: false,
        }
    }

    pub fn unstack_current_depth(&mut self) {
        while self.program_stack.len() > 0 && self.program_stack.last().unwrap().depth == self.current_stack_depth {
            self.program_stack.pop();
        }
        self.current_stack_depth -= 1;
    }
}



#[derive(Debug, Clone)]
pub struct LocalReads {
    pub variables: Vec<String>,
}