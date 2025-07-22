use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::datatype::DataType,
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::{AlthreadError, AlthreadResult, ErrorType, Pos},
    parser::Rule,
    vm::instruction::{Instruction, InstructionType},
};

#[derive(Debug, Clone)]
pub struct ChannelDeclaration {
    pub ch_left_prog: String,
    pub ch_left_name: String,
    pub ch_right_prog: String,
    pub ch_right_name: String,
    pub datatypes: Vec<DataType>,
    // todo: direction
}

impl NodeBuilder for ChannelDeclaration {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let mut left_pairs = pairs.next().unwrap().into_inner();
        let left_prog = String::from(left_pairs.next().unwrap().as_str());
        let left_name = String::from(left_pairs.next().unwrap().as_str());

        let mut datatypes: Vec<DataType> = Vec::new();

        let types_pair = pairs.next();
        for pair in types_pair.unwrap().into_inner() {
            let datatype = DataType::from_str(pair.as_str());
            datatypes.push(datatype);
        }

        let mut right_pairs = pairs.next().unwrap().into_inner();
        let right_prog = String::from(right_pairs.next().unwrap().as_str());
        let right_name = String::from(right_pairs.next().unwrap().as_str());

        Ok(Self {
            ch_left_prog: left_prog,
            ch_left_name: left_name,
            ch_right_prog: right_prog,
            ch_right_name: right_name,
            datatypes,
        })
    }
}

fn get_var_id(
    var_name: &str,
    state: &mut CompilerState,
    pos: &Pos,
) -> AlthreadResult<Option<usize>> {
    if var_name == "self" {
        return Ok(None);
    }
    let var_idx = state
        .program_stack
        .iter()
        .rev()
        .position(|var| var.name == var_name)
        .ok_or(AlthreadError::new(
            ErrorType::VariableError,
            Some(*pos),
            format!("Variable '{}' not found", var_name),
        ))?;

    Ok(Some(var_idx))
}

fn get_prog_name(var_name: &str, state: &mut CompilerState, pos: &Pos) -> AlthreadResult<String> {
    let n = state.program_stack.len();
    if let Some(var_idx) = get_var_id(var_name, state, pos)? {
        match &state.program_stack.get(n - var_idx - 1).unwrap().datatype {
            DataType::Process(n) => Ok(n.clone()),
            _ => {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(*pos),
                    format!(
                        "Variable '{}' is not a process (found {})",
                        var_name,
                        state.program_stack.get(var_idx).unwrap().datatype
                    ),
                ))
            }
        }
    } else {
        return Ok(state.current_program_name.clone());
    }
}



impl InstructionBuilder for Node<ChannelDeclaration> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let dec = &self.value;

        let left_prog = get_prog_name(&dec.ch_left_prog, state, &self.pos)?;
        let right_prog = get_prog_name(&dec.ch_right_prog, state, &self.pos)?;

        // check if a channel with the same name already exists on this program
        let left_key = (left_prog.clone(), dec.ch_left_name.clone());
        
        // CLONE the undefined channels data to avoid holding a reference
        let left_undefined = state.undefined_channels().get(&left_key).cloned();
        if let Some(used) = left_undefined {
            state.undefined_channels_mut().remove(&left_key);
            if used.0 != dec.datatypes {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos),
                    format!(
                        "Channel declared with types ({}) but used with different types at line {}",
                        dec.datatypes
                            .iter()
                            .map(|d| d.to_string())
                            .collect::<Vec<_>>()
                            .join(","),
                        used.1.line
                    ),
                ));
            }
        }
        
        // CLONE the channels data to avoid holding a reference
        let left_channel_info = state.channels().get(&left_key).cloned();
        if let Some((datatypes, pos)) = left_channel_info {
            // check if the datatypes are the same
            if datatypes != dec.datatypes {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos),
                    format!(
                        "Channel already attached to program '{}' with different types at line {}",
                        left_prog, pos.line
                    ),
                ));
            }
        } else {
            state
                .channels_mut()
                .insert(left_key, (dec.datatypes.clone(), self.pos.clone()));
        }

        let right_key = (right_prog.clone(), dec.ch_right_name.clone());
        
        // CLONE the undefined channels data to avoid holding a reference
        let right_undefined = state.undefined_channels().get(&right_key).cloned();
        if let Some(used) = right_undefined {
            state.undefined_channels_mut().remove(&right_key);
            if used.0 != dec.datatypes {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos),
                    format!(
                        "Channel declared with types ({}) but used with different types at line {}",
                        dec.datatypes
                            .iter()
                            .map(|d| d.to_string())
                            .collect::<Vec<_>>()
                            .join(","),
                        used.1.line
                    ),
                ));
            }
        }
        
        // CLONE the channels data to avoid holding a reference
        let right_channel_info = state.channels().get(&right_key).cloned();
        if let Some((datatypes, pos)) = right_channel_info {
            // check if the datatypes are the same
            if datatypes != dec.datatypes {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos),
                    format!(
                        "Channel already attached to program '{}' with different types at line {}",
                        right_prog, pos.line
                    ),
                ));
            }
        } else {
            state
                .channels_mut()
                .insert(right_key, (dec.datatypes.clone(), self.pos.clone()));
        }

        Ok(InstructionBuilderOk::from_instructions(vec![Instruction {
            control: InstructionType::Connect {
                sender_pid: get_var_id(&dec.ch_left_prog, state, &self.pos)?,
                receiver_pid: get_var_id(&dec.ch_right_prog, state, &self.pos)?,
                sender_channel: dec.ch_left_name.clone(),
                receiver_channel: dec.ch_right_name.clone(),
            },
            pos: Some(self.pos),
        }]))
    }
}

impl AstDisplay for ChannelDeclaration {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}channel decl")?;

        Ok(())
    }
}
