use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder}, token::{datatype::DataType, literal::Literal},
    }, compiler::{CompilerState, Variable}, error::{AlthreadError, AlthreadResult, ErrorType}, no_rule, parser::Rule, vm::instruction::{Instruction, InstructionType, JumpIfControl, UnstackControl}
};

use super::{waiting_case::WaitDependency, Statement};


#[derive(Debug, Clone)]
pub struct ReceiveStatement {
    pub channel: String,
    pub variables: Vec<String>,
    pub statement: Option<Node<Statement>>,
}

impl NodeBuilder for ReceiveStatement {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        
        let mut pair = pairs.next().unwrap();

        let mut channel = "".to_string();

        if pair.as_rule() == Rule::object_identifier {
            channel = pair.as_str().to_string();
            pair = pairs.next().unwrap();
        }

        if pair.as_rule() != Rule::pattern_list {
            return Err(no_rule!(pair, "ReceiveStatement"))
        }

        let mut variables = Vec::new();
        let sub_pairs: Pairs<'_, Rule> = pair.into_inner();
        for pair in sub_pairs {
            variables.push(String::from(pair.as_str()));
        }


        let statement = match pairs.next() {
            Some(p) => Some(Node::build(p)?),
            None => None,
        };


        Ok(Self { channel, variables, statement })
    }
}

impl ReceiveStatement {
    pub fn add_dependencies(&self, dependencies: &mut WaitDependency) {
        dependencies.variables.extend(self.variables.clone());
        dependencies.channels_state.insert(self.channel.clone());
    }
}

impl InstructionBuilder for Node<ReceiveStatement> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {

        // The goal is to simulate a boolean expression so that, if it is false, then the stack contains only 
        // a false value, and if it is true, then the stack contains all the read variables from the channel and a true value.
        let channel_name =  self.value.channel.clone();


        // Channel peek either push all the values and a true value, or just a false value
        // here add all the variables to the stack and remove them only in the branch where the boolean is true
        for variable in &self.value.variables {
            state.program_stack.push(Variable {
                mutable: true,
                name: variable.clone(),
                datatype: DataType::Integer,
                depth: state.current_stack_depth,
            })
        }
        // Here we could add a boolean to the stack but if you look at the instructions below, we will remove it anyway


        let guard_instructions = vec![Instruction{ 
            control: InstructionType::Push(Literal::Bool(true)), //to be replaced by the evaluation of the guard condition
            pos: Some(self.pos),
        }];
        state.program_stack.push(Variable { // to be removed when the guard is implemented
            mutable: false,
            name: "".to_string(),
            datatype: DataType::Boolean,
            depth: state.current_stack_depth,
        });

        // check if the top of the stack is a boolean
        if state.program_stack.last().expect("stack should contain a value after an expression is compiled").datatype != DataType::Boolean {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(self.pos),
                "guard condition must be a boolean".to_string()
            ));
        }
        state.program_stack.pop();

        let statement_instructions = match &self.value.statement {
            Some(statement) => statement.compile(state)?,
            None => Vec::new(),
        };


        let mut instructions = Vec::new();

        instructions.push(Instruction{ 
            control: InstructionType::ChannelPeek(channel_name.clone()),
            pos: Some(self.pos),
        });

        instructions.push(Instruction{ 
            control: InstructionType::JumpIf(JumpIfControl {
                jump_false: 7 + (guard_instructions.len() + statement_instructions.len()) as i64, // If the channel is empty, jump to the end
                unstack_len: 0, // we keep the false value on the stack
            }),
            pos: Some(self.pos),
        });

        instructions.push(Instruction{ 
            control: InstructionType::Unstack(UnstackControl {
                unstack_len: 1, // we remove the true value on the stack (it will be replaced by the next expression
            }),
            pos: Some(self.pos),
        });
        
        instructions.extend(guard_instructions);

        instructions.push(Instruction{ 
            control: InstructionType::JumpIf(JumpIfControl {
                jump_false: 5 + statement_instructions.len() as i64, // If the guard is false, jump to the end
                unstack_len: 0, // keep the boolean of the guard on the stack
            }),
            pos: Some(self.pos),
        });

        instructions.push(Instruction{ 
            control: InstructionType::Unstack(UnstackControl {
                unstack_len: 1, // remove the boolean of the guard from the stack (but keep the variables used in the statement)
            }),
            pos: Some(self.pos),
        });
        instructions.push(Instruction{ 
            control: InstructionType::ChannelPop(channel_name.clone()), // actually do pop the channel
            pos: Some(self.pos),
        });
        instructions.extend(statement_instructions);

        instructions.push(Instruction{ 
            control: InstructionType::Unstack(UnstackControl {
                unstack_len: self.value.variables.len(), // remove the variables from the stack
            }),
            pos: Some(self.pos),
        }); 
        for _ in 0..self.value.variables.len() { state.program_stack.pop(); }

        instructions.push(Instruction{ 
            control: InstructionType::Push(Literal::Bool(true)), // the statement is finished, the global condition is a success
            pos: Some(self.pos),
        }); 

        // In all the branches above, a boolean is pushed on the stack:
        state.program_stack.push(Variable {
            mutable: false,
            name: "".to_string(),
            datatype: DataType::Boolean,
            depth: state.current_stack_depth,
        });
        // The next instruction will likely be a wait or an if based on the current stack top.

        
        Ok(instructions)
    }
}


impl AstDisplay for ReceiveStatement {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}receive")?;
        let pref = prefix.add_branch();
        writeln!(f, "{pref} channel '{}'", self.channel)?;
        let pref = prefix.add_branch();
        writeln!(f, "{pref} patterns ({})", self.variables.iter().map(|v| v.clone()).collect::<Vec<String>>().join(","))?;

        Ok(())
    }
}
