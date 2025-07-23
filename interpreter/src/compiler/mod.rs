use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

pub mod stdlib;

use crate::ast::statement::expression::LocalExpressionNode;
use crate::error::Pos;
use crate::vm::instruction::{Instruction};
use crate::{
    ast::token::{datatype::DataType, literal::Literal, identifier::Identifier},
    vm::instruction::ProgramCode,
};


#[derive(Debug, Clone)]
pub struct FunctionDefinition { 
    pub name: String,
    pub arguments: Vec<(Identifier, DataType)>,
    pub return_type: DataType,
    pub body: Vec<Instruction>,
    pub pos: Pos,
}

#[derive(Debug)]
pub struct InstructionBuilderOk {
    pub instructions: Vec<Instruction>,

    /// The indexes of the break instructions
    /// the key is the label of the loop to break
    pub break_indexes: HashMap<String, Vec<usize>>,

    /// The indexes of the continue instructions
    /// the key is the label of the loop to continue
    pub continue_indexes: HashMap<String, Vec<usize>>,

    /// The indexes of the return instructions
    pub return_indexes: Vec<usize>,
}

impl InstructionBuilderOk {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            break_indexes: HashMap::new(),
            continue_indexes: HashMap::new(),
            return_indexes: Vec::new(),
        }
    }
    pub fn from_instructions(instructions: Vec<Instruction>) -> Self {
        Self {
            instructions,
            break_indexes: HashMap::new(),
            continue_indexes: HashMap::new(),
            return_indexes: Vec::new(),
        }
    }
    pub fn extend(&mut self, other: Self) {
        let off_set = self.instructions.len();
        self.instructions.extend(other.instructions);

        for (k, v) in other.break_indexes.iter() {
            self.break_indexes
                .entry(k.clone())
                .or_insert_with(Vec::new)
                .extend(v.iter().map(|x| x + off_set));
        }

        for (k, v) in other.continue_indexes.iter() {
            self.continue_indexes
                .entry(k.clone())
                .or_insert_with(Vec::new)
                .extend(v.iter().map(|x| x + off_set));
        }

        self.return_indexes
            .extend(other.return_indexes.iter().map(|x| x + off_set));

    }
    pub fn contains_jump(&self) -> bool {
        self.break_indexes.len() > 0
            || self.continue_indexes.len() > 0
            || self.return_indexes.len() > 0
    }
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub mutable: bool,
    pub name: String,
    pub datatype: DataType,
    pub depth: usize,
    pub declare_pos: Option<Pos>,
}


#[derive(Debug)]
pub struct CompilationContext {
    pub stdlib: Rc<stdlib::Stdlib>,
    pub user_functions: HashMap<String, FunctionDefinition>,
    pub global_table: HashMap<String, Variable>,
    pub program_arguments: HashMap<String, Vec<DataType>>,
    pub programs_code: HashMap<String, ProgramCode>,
    pub global_memory: BTreeMap<String, Literal>,
    
    // Add channel state
    pub channels: HashMap<(String, String), (Vec<DataType>, Pos)>,
    pub undefined_channels: HashMap<(String, String), (Vec<DataType>, Pos)>,

    // add always and eventually conditions
    pub always_conditions: Vec<(HashSet<String>, Vec<String>, LocalExpressionNode, Pos)>,
    pub eventually_conditions: Vec<(HashSet<String>, Vec<String>, LocalExpressionNode, Pos)>,
}

impl CompilationContext {
    pub fn new(stdlib: Rc<stdlib::Stdlib>) -> Self {
        Self {
            stdlib,
            user_functions: HashMap::new(),
            global_table: HashMap::new(),
            program_arguments: HashMap::new(),
            global_memory: BTreeMap::new(),
            channels: HashMap::new(),
            undefined_channels: HashMap::new(),
            always_conditions: Vec::new(),
            eventually_conditions: Vec::new(),
            programs_code: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct CompilerState {
    pub program_stack: Vec<Variable>,
    pub current_stack_depth: usize,
    pub current_program_name: String,
    pub is_atomic: bool,
    pub is_shared: bool,
    pub in_function: bool,
    pub method_call_stack_offset: usize,
    
    // Reference to shared context
    pub context: Rc<RefCell<CompilationContext>>,
}

impl CompilerState {
    pub fn new_with_context(context: Rc<RefCell<CompilationContext>>) -> Self {
        Self {
            program_stack: Vec::new(),
            current_stack_depth: 0,
            current_program_name: String::new(),
            is_atomic: false,
            is_shared: false,
            in_function: false,
            method_call_stack_offset: 0,
            context,
        }
    }

    pub fn stdlib(&self) -> Rc<stdlib::Stdlib> {
        self.context.borrow().stdlib.clone()
    }

    pub fn stdlib_mut(&mut self) -> Rc<stdlib::Stdlib> {
        Rc::clone(&self.context.borrow_mut().stdlib)
    }
    
    pub fn user_functions(&self) -> std::cell::Ref<HashMap<String, FunctionDefinition>> {
        std::cell::Ref::map(self.context.borrow(), |ctx| &ctx.user_functions)
    }
    
    pub fn user_functions_mut(&self) -> std::cell::RefMut<HashMap<String, FunctionDefinition>> {
        std::cell::RefMut::map(self.context.borrow_mut(), |ctx| &mut ctx.user_functions)
    }
    
    pub fn global_table(&self) -> std::cell::Ref<HashMap<String, Variable>> {
        std::cell::Ref::map(self.context.borrow(), |ctx| &ctx.global_table)
    }
    
    pub fn global_table_mut(&self) -> std::cell::RefMut<HashMap<String, Variable>> {
        std::cell::RefMut::map(self.context.borrow_mut(), |ctx| &mut ctx.global_table)
    }

    pub fn global_memory(&self) -> std::cell::Ref<BTreeMap<String, Literal>> {
        std::cell::Ref::map(self.context.borrow(), |ctx| &ctx.global_memory)
    }  

    pub fn global_memory_mut(&self) -> std::cell::RefMut<BTreeMap<String, Literal>> {
        std::cell::RefMut::map(self.context.borrow_mut(), |ctx| &mut ctx.global_memory)
    }
    
    pub fn channels(&self) -> std::cell::Ref<HashMap<(String, String), (Vec<DataType>, Pos)>> {
        std::cell::Ref::map(self.context.borrow(), |ctx| &ctx.channels)
    }
    
    pub fn channels_mut(&self) -> std::cell::RefMut<HashMap<(String, String), (Vec<DataType>, Pos)>> {
        std::cell::RefMut::map(self.context.borrow_mut(), |ctx| &mut ctx.channels)
    }

    pub fn undefined_channels(&self) -> std::cell::Ref<HashMap<(String, String), (Vec<DataType>, Pos)>> {
        std::cell::Ref::map(self.context.borrow(), |ctx| &ctx.undefined_channels)
    }  
    
    pub fn undefined_channels_mut(&self) -> std::cell::RefMut<HashMap<(String, String), (Vec<DataType>, Pos)>> {
        std::cell::RefMut::map(self.context.borrow_mut(), |ctx| &mut ctx.undefined_channels)
    }

    pub fn program_arguments(&self) -> std::cell::Ref<HashMap<String, Vec<DataType>>> {
        std::cell::Ref::map(self.context.borrow(), |ctx| &ctx.program_arguments)
    }

    pub fn program_arguments_mut(&self) -> std::cell::RefMut<HashMap<String, Vec<DataType>>> {
        std::cell::RefMut::map(self.context.borrow_mut(), |ctx| &mut ctx.program_arguments)
    }

    pub fn always_conditions(&self) -> std::cell::Ref<Vec<(HashSet<String>, Vec<String>, LocalExpressionNode, Pos)>> {
        std::cell::Ref::map(self.context.borrow(), |ctx| &ctx.always_conditions)
    }

    pub fn always_conditions_mut(&self) -> std::cell::RefMut<Vec<(HashSet<String>, Vec<String>, LocalExpressionNode, Pos)>> {
        std::cell::RefMut::map(self.context.borrow_mut(), |ctx| &mut ctx.always_conditions)
    }

    pub fn eventually_conditions(&self) -> std::cell::Ref<Vec<(HashSet<String>, Vec<String>, LocalExpressionNode, Pos)>> {
        std::cell::Ref::map(self.context.borrow(), |ctx| &ctx.eventually_conditions)
    }

    pub fn eventually_conditions_mut(&self) -> std::cell::RefMut<Vec<(HashSet<String>, Vec<String>, LocalExpressionNode, Pos)>> {
        std::cell::RefMut::map(self.context.borrow_mut(), |ctx| &mut ctx.eventually_conditions)
    }

    pub fn programs_code(&self) -> std::cell::Ref<HashMap<String, ProgramCode>> {
        std::cell::Ref::map(self.context.borrow(), |ctx| &ctx.programs_code)
    }

    pub fn programs_code_mut(&self) -> std::cell::RefMut<HashMap<String, ProgramCode>> {
        std::cell::RefMut::map(self.context.borrow_mut(), |ctx| &mut ctx.programs_code)
    }

    /// Pop all variables from the program stack that have the same depth as the current stack depth
    /// and decrease the current stack depth by one.
    /// Returns the number of variables that were popped.
    pub fn unstack_current_depth(&mut self) -> usize {
        let mut unstack_len = 0;
        while self.program_stack.len() > 0
            && self.program_stack.last().unwrap().depth == self.current_stack_depth
        {
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
    pub program_arguments: HashMap<String, Vec<DataType>>,
    pub user_functions: HashMap<String, FunctionDefinition>,
    pub global_memory: BTreeMap<String, Literal>,
    pub global_table: HashMap<String, Variable>,

    /// The conditions that should always be true
    /// The first element is the variables that are used in the condition
    /// The second element is the two instructions that are used to check the condition
    /// (the first in struction is the read operation and the second is the expression)
    pub always_conditions: Vec<(HashSet<String>, Vec<String>, LocalExpressionNode, Pos)>,
    /// conditions that must be true at least once in each possible executions
    pub eventually_conditions: Vec<(HashSet<String>, Vec<String>, LocalExpressionNode, Pos)>,

    pub stdlib: Rc<stdlib::Stdlib>,
}

impl fmt::Display for CompiledProject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Project:")?;

        writeln!(f, "- shared memory:")?;
        for (k, v) in self.global_memory.iter() {
            writeln!(f, "{}: {}", k, v)?;
        }
        for (k, v) in self.programs_code.iter() {
            writeln!(f, "- program '{}':", k)?;
            writeln!(f, "{}", v)?;
        }
        Ok(())
    }
}
