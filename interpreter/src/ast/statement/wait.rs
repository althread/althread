use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::{
            binary_assignment_operator::BinaryAssignmentOperator, datatype::DataType,
            literal::Literal,
        },
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType},
    no_rule,
    parser::Rule,
    vm::instruction::{Instruction, InstructionType},
};

use super::waiting_case::{WaitDependency, WaitingBlockCase};

#[derive(Debug, Clone, PartialEq)]
pub enum WaitingBlockKind {
    First,
    Seq,
}

#[derive(Debug, Clone)]
pub struct Wait {
    pub block_kind: WaitingBlockKind,
    pub waiting_cases: Vec<Node<WaitingBlockCase>>,
    pub start_atomic: bool,
}

impl NodeBuilder for Wait {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        let mut block_kind = WaitingBlockKind::First;

        let waiting_cases = match pair.as_rule() {
            Rule::waiting_block => {
                let mut pair = pair.into_inner();
                block_kind = match pair.next().unwrap().as_rule() {
                    Rule::FIRST_KW => WaitingBlockKind::First,
                    Rule::SEQ_KW => WaitingBlockKind::Seq,
                    _ => unreachable!(),
                };
                let mut children = Vec::new();
                for sub_pair in pair {
                    let node: Node<WaitingBlockCase> = Node::build(sub_pair, filepath)?;
                    children.push(node);
                }
                children
            }
            Rule::waiting_block_case => {
                let node: Node<WaitingBlockCase> = Node::build(pair, filepath)?;
                vec![node]
            }
            _ => {
                return Err(no_rule!(pair, "Wait", filepath));
            }
        };

        Ok(Self {
            block_kind,
            waiting_cases,
            start_atomic: false,
        })
    }
}

impl InstructionBuilder for Node<Wait> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        if state.is_atomic {
            return Err(AlthreadError::new(
                ErrorType::InstructionNotAllowed,
                Some(self.pos.clone()),
                "Wait blocks cannot be inside an atomic block (except if it is the first instruction)".to_string(),
            ));
        }
        if self.value.start_atomic {
            state.is_atomic = true;
        }

        let mut builder = InstructionBuilderOk::new();

        let mut dependencies = WaitDependency::new();
        for wc in self.value.waiting_cases.iter() {
            wc.value.rule.add_dependencies(&mut dependencies);
        }

        builder.instructions.push(Instruction {
            pos: Some(self.pos.clone()),
            control: InstructionType::WaitStart {
                dependencies,
                start_atomic: self.value.start_atomic,
            },
        });

        state.program_stack.push(Variable {
            datatype: DataType::Boolean,
            name: "".to_string(),
            mutable: true,
            depth: state.current_stack_depth,
            declare_pos: None,
        });

        builder.instructions.push(Instruction {
            pos: Some(self.pos.clone()),
            control: InstructionType::Push(Literal::Bool(false)),
        });

        let mut jump_if_offset = if self.value.block_kind == WaitingBlockKind::First {
            1
        } else {
            0
        };
        // the if offset also depends on whether the await block is atomic or not
        if !self.value.start_atomic {
            jump_if_offset += 1;
        }

        let mut jump_index = Vec::new();
        for case in &self.value.waiting_cases {
            state.current_stack_depth += 1;
            let mut case_condition = case.value.rule.compile(state)?;
            let unstack_len = state.unstack_current_depth();

            let mut case_statement = match &case.value.statement {
                Some(s) => s.compile(state)?,
                None => InstructionBuilderOk::new(),
            };

            case_statement.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::Push(Literal::Bool(true)),
            });
            case_statement.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::LocalAssignment {
                    index: 0,
                    operator: BinaryAssignmentOperator::OrAssign,
                    unstack_len: 1,
                },
            });

            // the offset is because a jump will be added after the statement
            case_condition.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::JumpIf {
                    jump_false: (case_statement.instructions.len() + 1 + jump_if_offset) as i64,
                    unstack_len,
                },
            });
            builder.extend(case_condition);
            if !self.value.start_atomic {
                // if the entire wait block is not atomic, stop the atomicity here
                builder.instructions.push(Instruction {
                    pos: Some(case.pos.clone()),
                    control: InstructionType::AtomicEnd,
                });
            }
            builder.extend(case_statement);
            jump_index.push(builder.instructions.len());
            builder.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::Empty, // placeholder for the jump if the keyword "first" is used
            });
        }

        builder.instructions.push(Instruction {
            pos: Some(self.pos.clone()),
            control: InstructionType::Wait {
                jump: -(builder.instructions.len() as i64),
                unstack_len: 1,
            },
        });
        state.program_stack.pop();

        if self.value.block_kind == WaitingBlockKind::First {
            for index in jump_index.iter() {
                builder.instructions[*index].control =
                    InstructionType::Jump((builder.instructions.len() - index - 1) as i64);
            }
        }

        /*
                state.current_stack_depth += 1;
                let cond_ins = self.value.condition.compile(state)?;
                // Check if the top of the stack is a boolean
                if state.program_stack.last().expect("stack should contain a value after an expression is compiled").datatype != DataType::Boolean {
                    return Err(AlthreadError::new(
                        ErrorType::TypeError,
                        Some(self.value.condition.pos),
                        "condition must be a boolean".to_string()
                    ));
                }
                // pop all variables from the stack at the given depth
                let unstack_len = state.unstack_current_depth();

                instructions.extend(cond_ins);

                instructions.push(Instruction {
                    pos: Some(self.pos),
                    control: InstructionType::Wait(WaitControl {
                        jump: -(instructions.len() as i64),
                        unstack_len,
                    }),
                });
        */
        // It should work!
        //if builder.contains_jump() {
        //    unimplemented!("breaks in await blocks are not yet implemented");
        //}
        Ok(builder)
    }
}

impl AstDisplay for Wait {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}wait_control")?;
        {
            for case in &self.waiting_cases {
                case.ast_fmt(f, &prefix.add_leaf())?;
            }
        }

        Ok(())
    }
}
