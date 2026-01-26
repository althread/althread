use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::{datatype::DataType, literal::Literal, object_identifier::ObjectIdentifier},
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType},
    no_rule,
    parser::Rule,
    vm::instruction::{Instruction, InstructionType},
};

use super::waiting_case::WaitDependency;

#[derive(Debug, Clone)]
pub struct ReceiveStatement {
    pub channel: String,
    pub variables: Vec<String>,
}

impl NodeBuilder for ReceiveStatement {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let mut pair = pairs.next().unwrap();

        let mut channel = "".to_string();

        if pair.as_rule() == Rule::object_identifier {
            // Parse the object_identifier and convert it to a string
            let object_id = Node::<ObjectIdentifier>::build(pair, filepath)?;
            channel = object_id
                .value
                .parts
                .iter()
                .map(|p| p.value.value.as_str())
                .collect::<Vec<_>>()
                .join(".");
            pair = pairs.next().unwrap();
        }

        if pair.as_rule() != Rule::pattern_list {
            return Err(no_rule!(pair, "ReceiveStatement", filepath));
        }

        let mut variables = Vec::new();
        let sub_pairs: Pairs<'_, Rule> = pair.into_inner();
        for pair in sub_pairs {
            variables.push(String::from(pair.as_str()));
        }

        Ok(Self {
            channel,
            variables,
        })
    }
}

impl ReceiveStatement {
    pub fn add_dependencies(&self, dependencies: &mut WaitDependency) {
        dependencies.variables.extend(self.variables.clone());
        dependencies.channels_state.insert(self.channel.clone());
    }
}

impl InstructionBuilder for Node<ReceiveStatement> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        // The goal is to simulate a boolean expression so that, if it is false, then the stack contains only
        // a false value, and if it is true, then the stack contains all the read variables from the channel and a true value.
        let channel_name = self.value.channel.clone();

        log::debug!("channels: {:?}", state.channels().clone());

        // first check that the correct number of variables are supplied
        // retreive the variable from the declared channel:
        let (channel_types, pos) = state.channels().get(&(state.current_program_name.clone(), channel_name.clone())).ok_or(AlthreadError::new(
            ErrorType::TypeError,
            Some(self.pos.clone()),
            format!("Cannot infer the types of the channel '{}', please declare the channel (even if not used)", channel_name)
        ))?.clone();

        // check that the number of variables is correct
        if channel_types.len() != self.value.variables.len() {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(self.pos.clone()),
                format!(
                    "Channel {}, bound at line {}, expects {} values, but {} variables are given",
                    self.value.channel,
                    pos.line,
                    channel_types.len(),
                    self.value.variables.len()
                ),
            ));
        }

        let mut builder = InstructionBuilderOk::new();

        builder.instructions.push(Instruction {
            control: InstructionType::ChannelPeek(channel_name.clone()),
            pos: Some(self.pos.clone()),
        }); // Peek has the effect of adding the values of the tuple to the stack and a boolean
        // We must push default values if the channel is empty, just so that the stack is consistent

        for (i, variable) in self.value.variables.iter().enumerate() {
            state.program_stack.push(Variable {
                mutable: true,
                name: variable.clone(),
                datatype: channel_types[i].clone(),
                depth: state.current_stack_depth,
                declare_pos: Some(pos.clone()),
            })
        }

        state.program_stack.push(Variable {
            mutable: false,
            name: "".to_string(),
            datatype: DataType::Boolean,
            depth: state.current_stack_depth,
            declare_pos: None,
        });

        builder.instructions.push(Instruction {
            control: InstructionType::JumpIf {
                jump_false: 3, // If the channel is empty, ignore the channel pop
                unstack_len: 0, // we keep the boolean value on the stack
            },
            pos: Some(self.pos.clone()),
        });

        builder.instructions.push(Instruction {
            control: InstructionType::ChannelPop(channel_name.clone()), // actually do pop the channel
            pos: Some(self.pos.clone()),
        });
        // now we jump over the push of default values
        builder.instructions.push(Instruction {
            control: InstructionType::Jump (5),
            pos: Some(self.pos.clone()),
        });
        // remove the false boolean
        builder.instructions.push(Instruction {
            control: InstructionType::Unstack{
                unstack_len: 1,
            },
            pos: Some(self.pos.clone()),
        });
        // push default values, by pushing a tuple and the descructuring it
        builder.instructions.push(Instruction {
            control: InstructionType::Push(Literal::Tuple(
                channel_types
                    .iter()
                    .map(|dt| dt.default())
                    .collect(),
            )),
            pos: Some(self.pos.clone()),
        });
        builder.instructions.push(Instruction {
            control: InstructionType::Destruct,
            pos: Some(pos),
        });
        //repush the false boolean
        builder.instructions.push(Instruction {
            control: InstructionType::Push(Literal::Bool(false)),
            pos: Some(self.pos.clone()),
        });
        
        Ok(builder)
    }
}

impl AstDisplay for ReceiveStatement {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}receive")?;
        let pref = prefix.add_branch();
        writeln!(f, "{pref} channel '{}'", self.channel)?;
        let pref = prefix.add_branch();
        writeln!(
            f,
            "{pref} patterns ({})",
            self.variables
                .iter()
                .map(|v| v.clone())
                .collect::<Vec<String>>()
                .join(",")
        )?;

        Ok(())
    }
}
