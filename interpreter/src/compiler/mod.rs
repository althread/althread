use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::error::Pos;
use crate::vm::instruction::{ExpressionControl, GlobalReadsControl, Instruction};
use crate::{ast::token::{datatype::DataType, literal::Literal}, vm::instruction::ProgramCode};

#[derive(Debug, Clone)]
pub struct Variable {
    pub mutable: bool,
    pub name: String,
    pub datatype: DataType,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct CompilerState {
    pub global_table: HashMap<String, Variable>,
    pub program_stack: Vec<Variable>,
    pub current_stack_depth: usize,
    pub is_atomic: bool,
}

impl CompilerState {
    pub fn new() -> Self {
        Self {
            global_table: HashMap::new(),
            program_stack: Vec::new(),
            current_stack_depth: 0,
            is_atomic: false,
        }
    }

    pub fn unstack_current_depth(&mut self) -> usize {
        let mut unstack_len = 0;
        while self.program_stack.len() > 0 && self.program_stack.last().unwrap().depth == self.current_stack_depth {
            self.program_stack.pop();
            unstack_len += 1;
        }
        self.current_stack_depth -= 1;
        unstack_len
    }
}

#[derive(Debug)]
pub struct CompiledProject {
    pub programs_code: HashMap<String, ProgramCode>,
    pub global_memory: HashMap<String, Literal>,

    /// The conditions that should always be true
    /// The first element is the variables that are used in the condition
    /// The second element is the two instructions that are used to check the condition 
    /// (the first in struction is the read operation and the second is the expression)
    pub always_conditions: Vec<(HashSet<String>, GlobalReadsControl, ExpressionControl, Pos)>,
}


impl fmt::Display for CompiledProject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Project:")?;

        writeln!(f, "- shared memory:")?;
        for (k, v) in self.global_memory.iter() {
            writeln!(f, "{}: {:?}", k, v)?;
        };
        for (k, v) in self.programs_code.iter() {
            writeln!(f, "- program '{}':", k)?;
            writeln!(f, "{}", v)?;
        };
        Ok(())
    }
}