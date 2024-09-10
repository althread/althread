use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
    }, compiler::CompilerState, error::{AlthreadError, AlthreadResult, ErrorType}, parser::Rule, vm::instruction::{Instruction, InstructionType, SendControl}
};

use super::expression::Expression;

#[derive(Debug, Clone)]
pub struct SendStatement {
    pub channel: String,
    pub values: Vec<Node<Expression>>,
}

impl NodeBuilder for SendStatement {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        
        let channel = String::from(pairs.next().unwrap().as_str());

        let mut values = Vec::new();
        while let Some(pair) = pairs.next() {
            values.push(Node::build(pair)?);
        }

        Ok(Self { channel, values })
    }
}

impl InstructionBuilder for Node<SendStatement> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {
        let channel_name =  self.value.channel.clone();

        let mut instructions = Vec::new();
        
        state.current_stack_depth += 1;
        instructions.append(&mut self.value.values[0].compile(state)?);
        let rdatatype = state.program_stack.last().expect("empty stack after expression").datatype.clone();
        let unstack_len = state.unstack_current_depth();

        
        if state.channels.get(&(state.current_program_name.clone(), channel_name.clone())).is_none() {
            state.undefined_channels.insert((state.current_program_name.clone(), channel_name.clone()), (vec![rdatatype], self.pos));
        } else {

            let (channel_types, pos) = state.channels.get(&(state.current_program_name.clone(), self.value.channel.clone())).unwrap();
            if channel_types.len() != self.value.values.len() {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos),
                    format!("Channel {}, bound at line {}, expects {} values, but {} were given", 
                    self.value.channel, pos.line, channel_types.len(), self.value.values.len())
                ))
            }

            if channel_types[0] != rdatatype {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos),
                    format!("Channel {}, bound at line {}, expects values of types {}, but {} were given", self.value.channel, pos.line, channel_types[0], rdatatype)
                ))
            }
        }

        instructions.push(Instruction {
            control:InstructionType::Send(SendControl {
                channel_name,
                nb_values: self.value.values.len(),
                unstack_len
            }), 
            pos: Some(self.pos),
        });
        Ok(instructions)
    }
}


impl AstDisplay for SendStatement {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}send")?;
        prefix.add_leaf();
        writeln!(f, "{prefix}{}", self.channel)?;

        Ok(())
    }
}
