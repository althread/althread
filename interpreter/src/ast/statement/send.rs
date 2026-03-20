use std::fmt;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node},
        token::datatype::DataType,
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::{AlthreadError, AlthreadResult, ErrorType},
    vm::instruction::{Instruction, InstructionType},
};

use super::expression::Expression;

#[derive(Debug, Clone)]
pub struct SendStatement {
    pub channel: String,
    pub is_broadcast: bool,
    pub values: Node<Expression>,
}

impl InstructionBuilder for Node<SendStatement> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let channel_name = self.value.channel.clone();

        let mut builder = InstructionBuilderOk::new();

        let tuple = match &self.value.values.value {
            Expression::Tuple(t) => &t.value,
            _ => {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos.clone()),
                    "Send statement expects a tuple of values".to_string(),
                ))
            }
        };

        state.current_stack_depth += 1;
        builder.extend(self.value.values.compile(state)?);
        let rdatatype = state
            .program_stack
            .last()
            .expect("empty stack after expression")
            .datatype
            .clone();
        let unstack_len = state.unstack_current_depth();

        if self.value.is_broadcast {
            let mut target_channels: Vec<_> = state
                .channels()
                .iter()
                .filter(|((prog, name), _)| {
                    *prog == state.current_program_name
                        && name.starts_with(&(channel_name.clone() + "."))
                })
                .map(|((_, name), (types, pos))| (name.clone(), types.clone(), pos.clone()))
                .collect();

            if target_channels.is_empty() {
                builder.instructions.push(Instruction {
                    control: InstructionType::Broadcast {
                        channel_name: channel_name.clone() + ".",
                        unstack_len,
                    },
                    pos: Some(self.pos.clone()),
                });
            } else {
                // sort for deterministic compilation
                target_channels.sort_by(|a, b| a.0.cmp(&b.0));

                for (i, (ch_name, ch_types, pos)) in target_channels.iter().enumerate() {
                    if ch_types.len() != tuple.values.len() {
                        return Err(AlthreadError::new(
                            ErrorType::TypeError,
                            Some(self.pos.clone()),
                            format!(
                                "Channel {}, bound at line {}, expects {} values, but {} were given",
                                ch_name,
                                pos.line,
                                ch_types.len(),
                                tuple.values.len()
                            ),
                        ));
                    }
                    let channel_types = DataType::Tuple(ch_types.clone());
                    if channel_types != rdatatype {
                        return Err(AlthreadError::new(
                            ErrorType::TypeError,
                            Some(self.pos.clone()),
                            format!(
                                "Channel {}, bound at line {}, expects values of types {}, but {} were given",
                                ch_name, pos.line, channel_types, rdatatype
                            ),
                        ));
                    }

                    // If it is the last channel, we unstack the values
                    let current_unstack = if i == target_channels.len() - 1 {
                        unstack_len
                    } else {
                        0
                    };

                    builder.instructions.push(Instruction {
                        control: InstructionType::Send {
                            channel_name: ch_name.clone(),
                            unstack_len: current_unstack,
                        },
                        pos: Some(self.pos.clone()),
                    });
                }
            }
        } else {
            let channel_info = state
                .channels()
                .get(&(state.current_program_name.clone(), channel_name.clone()))
                .cloned();

            if channel_info.is_none() {
                state.undefined_channels_mut().insert(
                    (state.current_program_name.clone(), channel_name.clone()),
                    (vec![rdatatype], self.pos.clone()),
                );
            } else {
                let (channel_types, pos) = channel_info.unwrap();

                if channel_types.len() != tuple.values.len() {
                    return Err(AlthreadError::new(
                        ErrorType::TypeError,
                        Some(self.pos.clone()),
                        format!(
                            "Channel {}, bound at line {}, expects {} values, but {} were given",
                            self.value.channel,
                            pos.line,
                            channel_types.len(),
                            tuple.values.len()
                        ),
                    ));
                }

                let channel_types = DataType::Tuple(channel_types.clone());

                if channel_types != rdatatype {
                    return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos.clone()),
                    format!("Channel {}, bound at line {}, expects values of types {}, but {} were given", self.value.channel, pos.line, channel_types, rdatatype)
                ));
                }
            }

            builder.instructions.push(Instruction {
                control: InstructionType::Send {
                    channel_name: channel_name.clone(),
                    unstack_len,
                },
                pos: Some(self.pos.clone()),
            });
        }

        Ok(builder)
    }
}

impl AstDisplay for SendStatement {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}send")?;
        writeln!(f, "{}{}", prefix.add_branch(), self.channel)?;
        self.values.ast_fmt(f, &prefix.add_leaf())?;

        Ok(())
    }
}
