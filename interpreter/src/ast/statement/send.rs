use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::datatype::DataType,
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::{AlthreadError, AlthreadResult, ErrorType},
    parser::Rule,
    vm::instruction::{Instruction, InstructionType},
};

use super::expression::Expression;

#[derive(Debug, Clone)]
pub struct SendStatement {
    pub channel: String,
    pub values: Node<Expression>,
}

impl NodeBuilder for SendStatement {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let channel = String::from(pairs.next().unwrap().as_str());

        let values: Node<Expression> = Expression::build_top_level(pairs.next().unwrap())?;

        if !values.value.is_tuple() {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(values.pos),
                "Send statement expects a tuple of values".to_string(),
            ));
        }

        Ok(Self { channel, values })
    }
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
                    Some(self.pos),
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

        if state
            .channels
            .get(&(state.current_program_name.clone(), channel_name.clone()))
            .is_none()
        {
            state.undefined_channels.insert(
                (state.current_program_name.clone(), channel_name.clone()),
                (vec![rdatatype], self.pos),
            );
        } else {
            let (channel_types, pos) = state
                .channels
                .get(&(
                    state.current_program_name.clone(),
                    self.value.channel.clone(),
                ))
                .unwrap();

            if channel_types.len() != tuple.values.len() {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos),
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
                    Some(self.pos),
                    format!("Channel {}, bound at line {}, expects values of types {}, but {} were given", self.value.channel, pos.line, channel_types, rdatatype)
                ));
            }
        }

        builder.instructions.push(Instruction {
            control: InstructionType::Send {
                channel_name: channel_name.clone(),
                unstack_len,
            },
            pos: Some(self.pos),
        });

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
