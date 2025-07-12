use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::{datatype::DataType, object_identifier::ObjectIdentifier},
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType},
    parser::Rule,
    vm::instruction::{Instruction, InstructionType},
};

use super::expression::Expression;

#[derive(Debug, Clone)]
pub struct RunCall {
    pub identifier: Node<ObjectIdentifier>,
    pub args: Node<Expression>,
}

impl RunCall {
    pub fn program_name_to_string(&self) -> String {
        self.identifier
            .value
            .parts
            .iter()
            .map(|part| part.value.value.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }
}

impl NodeBuilder for RunCall {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let identifier = Node::build(pairs.next().unwrap())?;
        let args: Node<Expression> = Expression::build_top_level(pairs.next().unwrap())?;

        if !args.value.is_tuple() {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(args.pos),
                "Run statement expects a tuple of arguments (possibly empty)".to_string(),
            ));
        }
        Ok(Self { identifier, args })
    }
}

impl InstructionBuilder for Node<RunCall> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();
        let full_program_name = self.value.program_name_to_string();

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

        println!(
            "RunCall: {} with args {:?} at pos {:?}",
            full_program_name, call_datatype, self.pos
        );

        // CLONE the program arguments to avoid holding a reference
        let prog_args_opt = state.program_arguments().get(&full_program_name).cloned();

        println!("state program arguments: {:?}", state.program_arguments());

        if let Some(prog_args) = prog_args_opt {
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
                            i + 1,
                            arg,
                            call_datatype[i]
                        ),
                    ));
                }
            }
        } else {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(self.pos),
                format!("Program {} does not exist", full_program_name),
            ));
        }

        // Then call the function (this will add to the stack the pid of the new process)
        state.program_stack.push(Variable {
            name: "".to_string(),
            depth: state.current_stack_depth,
            mutable: false,
            datatype: DataType::Process(full_program_name.clone()),
            declare_pos: Some(self.pos),
        });

        builder.instructions.push(Instruction {
            control: InstructionType::RunCall {
                name: full_program_name,
                unstack_len,
            },
            pos: Some(self.pos),
        });

        Ok(builder)
    }
}

impl AstDisplay for RunCall {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        let program_name = self.program_name_to_string();
        writeln!(f, "{prefix}run: {}", program_name)?;

        Ok(())
    }
}
