use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::datatype::DataType,
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType, Pos},
    parser::Rule,
    vm::instruction::{Instruction, InstructionType, RunCallControl},
};

use super::expression::Expression;

#[derive(Debug, Clone)]
pub struct RunCall {
    pub identifier: Node<String>,
    pub args: Node<Expression>,
}

impl NodeBuilder for RunCall {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        let identifier = Node {
            pos: Pos {
                line: pair.line_col().0,
                col: pair.line_col().1,
                start: pair.as_span().start(),
                end: pair.as_span().end(),
            },
            value: pair.as_str().to_string(),
        };

        let args: Node<Expression> = Expression::build_top_level(pairs.next().unwrap())?;

        if !args.value.is_tuple() {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(args.pos),
                "Run statement expects a tuple of arguments (possibly empty)".to_string(),
            ));
        }
        println!("RunCall: {:?} {:?}", identifier, args);

        Ok(Self { identifier, args })
    }
}

impl InstructionBuilder for Node<RunCall> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {

        let mut builder = InstructionBuilderOk::new();


        // push the args to the stack
        state.current_stack_depth += 1;
        builder.extend(self.value.args.compile(state)?);
        let call_datatype = state
            .program_stack
            .last()
            .expect("empty stack after expression")
            .datatype
            .clone();
        let unstack_len = state.unstack_current_depth();
        let call_datatype = call_datatype.tuple_unwrap();

        if let Some(prog_args) = state.program_arguments.get(&self.value.identifier.value) {
            if prog_args.len() != call_datatype.len() {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos),
                    format!(
                        "Expected {} argument(s), got {}",
                        prog_args.len(),
                        call_datatype.len()
                    ),
                ));
            }
            for (i, arg) in prog_args.iter().enumerate() {
                if arg != &call_datatype[i] {
                    return Err(AlthreadError::new(
                        ErrorType::TypeError,
                        Some(self.pos),
                        format!(
                            "Expected argument {} to be of type {:?}, got {:?}",
                            i+1, arg, call_datatype[i]
                        ),
                    ));
                }
            }
        } else {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(self.pos),
                format!("Program {} does not exist", self.value.identifier.value),
            ));
        }

        // Then call the function (this will add to the stack the pid of the new process)
        state.program_stack.push(Variable {
            name: "".to_string(),
            depth: state.current_stack_depth,
            mutable: false,
            datatype: DataType::Process(self.value.identifier.value.clone()),
            declare_pos: Some(self.pos),
        });

        builder.instructions.push(Instruction {
            control: InstructionType::RunCall(RunCallControl {
                name: self.value.identifier.value.clone(),
                unstack_len,
            }),
            pos: Some(self.pos),
        });

        Ok(builder)

    }
}

impl AstDisplay for RunCall {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}run: {}", self.identifier)?;

        Ok(())
    }
}
