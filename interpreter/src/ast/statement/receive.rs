use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::{datatype::DataType, literal::Literal},
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType},
    no_rule,
    parser::Rule,
    vm::instruction::{Instruction, InstructionType, JumpIfControl, UnstackControl},
};

use super::{
    expression::tuple_expression::TupleExpression, waiting_case::WaitDependency, Statement,
};

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
            return Err(no_rule!(pair, "ReceiveStatement"));
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

        Ok(Self {
            channel,
            variables,
            statement,
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

        // first check that the correct number of variables are supplied
        // retreive the variable from the declared channel:
        let (channel_types, pos) = state.channels.get(&(state.current_program_name.clone(), channel_name.clone())).ok_or(AlthreadError::new(
            ErrorType::TypeError,
            Some(self.pos),
            format!("Cannot infer the types of the channel '{}', please declare the channel in the main (even if not used)", channel_name)
        ))?.clone();
        // check that the number of variables is correct
        if channel_types.len() != self.value.variables.len() {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(self.pos),
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
            pos: Some(self.pos),
        }); // Peek has the effect of adding an anonymous tuple to the stack
        state.program_stack.push(Variable {
            mutable: false,
            name: "".to_string(),
            datatype: DataType::Tuple(channel_types.clone()),
            depth: state.current_stack_depth,
            declare_pos: Some(self.pos),
        });

        // Channel peek either push all the values and a true value, or just a false value
        // here add all the variables to the stack and remove them only in the branch where the boolean is true

        let destruct_instruction = TupleExpression::destruct_tuple(
            &self.value.variables,
            &channel_types,
            state,
            self.pos,
        )?;
        // destructing remove the top of the stack and replace it with n values

        // Here we could add a boolean to the stack but if you look at the instructions below, we will remove it anyway
        let guard_instructions = vec![Instruction {
            control: InstructionType::Push(Literal::Bool(true)), //to be replaced by the evaluation of the guard condition
            pos: Some(self.pos),
        }];
        state.program_stack.push(Variable {
            // to be removed when the guard is implemented
            mutable: false,
            name: "".to_string(),
            datatype: DataType::Boolean,
            depth: state.current_stack_depth,
            declare_pos: None,
        });

        // check if the top of the stack is a boolean
        if state
            .program_stack
            .last()
            .expect("stack should contain a value after an expression is compiled")
            .datatype
            != DataType::Boolean
        {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(self.pos),
                "guard condition must be a boolean".to_string(),
            ));
        }
        state.program_stack.pop();

        let statement_builder = match &self.value.statement {
            Some(statement) => statement.compile(state)?,
            None => InstructionBuilderOk::new(),
        };

        builder.instructions.push(Instruction {
            control: InstructionType::JumpIf(JumpIfControl {
                jump_false: 8
                    + (guard_instructions.len() + statement_builder.instructions.len()) as i64, // If the channel is empty, jump to the end
                unstack_len: 0, // we keep the false value on the stack
            }),
            pos: Some(self.pos),
        });

        builder.instructions.push(Instruction {
            control: InstructionType::Unstack(UnstackControl {
                unstack_len: 1, // we remove the true value on the stack (it will be replaced by the next expression
            }),
            pos: Some(self.pos),
        });

        builder.instructions.push(destruct_instruction);

        builder.instructions.extend(guard_instructions);

        builder.instructions.push(Instruction {
            control: InstructionType::JumpIf(JumpIfControl {
                jump_false: 5 + statement_builder.instructions.len() as i64, // If the guard is false, jump to the end
                unstack_len: 0, // keep the boolean of the guard on the stack
            }),
            pos: Some(self.pos),
        });

        builder.instructions.push(Instruction {
            control: InstructionType::Unstack(UnstackControl {
                unstack_len: 1, // remove the boolean of the guard from the stack (but keep the variables used in the statement)
            }),
            pos: Some(self.pos),
        });
        builder.instructions.push(Instruction {
            control: InstructionType::ChannelPop(channel_name.clone()), // actually do pop the channel
            pos: Some(self.pos),
        });
        builder.extend(statement_builder);

        builder.instructions.push(Instruction {
            control: InstructionType::Unstack(UnstackControl {
                unstack_len: self.value.variables.len(), // remove the variables from the stack
            }),
            pos: Some(self.pos),
        });

        // removing the variables from the compiler stack (added in the destruct_tuple function)
        for _ in 0..self.value.variables.len() {
            state.program_stack.pop();
        }

        builder.instructions.push(Instruction {
            control: InstructionType::Push(Literal::Bool(true)), // the statement is finished, the global condition is a success
            pos: Some(self.pos),
        });

        // In all the branches above, a boolean is pushed on the stack:
        state.program_stack.push(Variable {
            mutable: false,
            name: "".to_string(),
            datatype: DataType::Boolean,
            depth: state.current_stack_depth,
            declare_pos: None,
        });
        // The next instruction will likely be a wait or an if based on the current stack top.

        //if builder.contains_jump() {
        //    todo!("breaking inside a receive statement is not yet implemented");
        //}

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
