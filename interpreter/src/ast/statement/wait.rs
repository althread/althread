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

        // We must treat the case when there is only one waiting case differently
        // Because in this case, if there is no statement following the condition
        // we keep the declared variables in the stack

        if self.value.waiting_cases.len() == 1 && self.value.waiting_cases[0].value.statement.is_none() {
            let case = &self.value.waiting_cases[0];

            // Here it is partucular, we only unstack these variables if the wait is not over
            // and we leave the stack unchanged if the wait is over
            // so we have to count how many variables were added by the condition without removing them
            let stack_size_before = state.program_stack.len();

            let case_condition = case.value.rule.compile(state)?;
            builder.extend(case_condition);

            // Remove the boolean from the compiler stack (it will be unstacked with the wait instruction at runtime)
            let boolean_var = state.program_stack.pop();
            debug_assert!(boolean_var.is_some() && boolean_var.as_ref().unwrap().datatype == DataType::Boolean);

            let unstack_len = state.program_stack.len() - stack_size_before;

            // If the condition is true, leave the atomic guard-evaluation phase.
            builder.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::JumpIf { 
                    jump_false: if self.value.start_atomic { 1 } else { 2 },
                    unstack_len: 0, // leave the boolean
                }
            });
            if !self.value.start_atomic {
                builder.instructions.push(Instruction {
                    pos: Some(case.pos.clone()),
                    control: InstructionType::AtomicEnd,
                });
            }
            builder.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::Jump(if self.value.start_atomic { 3 } else { 4 })
            });

            builder.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::Unstack { 
                    unstack_len: unstack_len + 1 //unstack the boolean as well
                },
            });
            // push a false to indicate that the wait was not over
            builder.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::Push(Literal::Bool(false)),
            });

            if !self.value.start_atomic {
                builder.instructions.push(Instruction {
                    pos: Some(case.pos.clone()),
                    control: InstructionType::AtomicEnd,
                });
            }

            builder.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::Wait {
                    jump: -(builder.instructions.len() as i64),
                    unstack_len: 1,
                },
            });
            // when the wait is over the variables declared in case are still on the stack and will be removed when the current scope ends
            // if the wait is not over, since there is only one case, we know that the variables declared will be eventually there.
            return Ok(builder)
        }


        /*
        In case we handle multiple waiting cases (or one with a statement), 
        we push a false boolean at the beginning, start atomicity if needed,
        then for each case:
        - condition instructions (adding a boolean and possibly other variables to the stack)
        - jump over the statement if the condition is false (and remove the boolean from the stack)
                    - stop atomicity instruction (if needed)
                    - statement instructions (possibly adding other variables to the stack)
          - unstack the instructions from the statement and condition variables
          - push true
          - jump over the unstacking of the condition variables
        -----
          - unstack the condition variables
          - push false
        - OrAssign to the variable at index 0 (after unstacking the top boolean)
        - add an empty instruction to be replaced if "first" is used
        
        Finally, we add:
        - wait instruction (jumping back to the beginning of the wait)
        */


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
        

        // Store the indexes of the success-path jumps to fill them later if
        // "first" is used. Failed cases must still fall through to later cases.
        let mut success_jump_index = Vec::new();
        for case in &self.value.waiting_cases {
            // In this case, the variables declared in the case condition are in their own scope
            state.current_stack_depth += 1;
            let case_condition = case.value.rule.compile(state)?;

            builder.extend(case_condition);
            
            // Remove the boolean from the compiler stack (it will be unstacked at runtime before the statement)
            let boolean_var = state.program_stack.pop();
            debug_assert!(boolean_var.is_some() && boolean_var.as_ref().unwrap().datatype == DataType::Boolean);
                        
            // a jumpIf instruction will be added between the condition and the statement, unstacking the boolean
            // but we need to compile the statement to know how many instructions to jump over
            // since we will have to unstack variables in the statement, we create a new depth for the statement
            state.current_stack_depth += 1;

            //  --- Statement compilation ---
            let mut case_statement = match &case.value.statement {
                Some(s) => s.compile(state)?,
                None => InstructionBuilderOk::new(),
            };

            if !self.value.start_atomic {
                case_statement.instructions.insert(
                    0,
                    Instruction {
                        pos: Some(case.pos.clone()),
                        control: InstructionType::AtomicEnd,
                    },
                );
            }

            // now we know hwo many variables were stacked the case condition and the statement
            let unstack_len_statement = state.unstack_current_depth();
            let unstack_len_condition = state.unstack_current_depth();

            case_statement.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::Unstack { 
                    unstack_len: unstack_len_statement + unstack_len_condition
                },
            });
            
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

            if self.value.block_kind == WaitingBlockKind::Seq && !self.value.start_atomic {
                // Re-enter atomic mode so the remaining seq cases are evaluated
                // as one uninterrupted guard-evaluation phase.
                case_statement.instructions.push(Instruction {
                    pos: Some(case.pos.clone()),
                    control: InstructionType::AtomicStart,
                });
            }

            case_statement.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: match self.value.block_kind {
                    WaitingBlockKind::First => {
                        // Placeholder patched to jump to the final wait check.
                        InstructionType::Empty
                    }
                    WaitingBlockKind::Seq => {
                        // Skip this case's failure cleanup and continue with the next case.
                        InstructionType::Jump(4)
                    }
                },
            });
            //  --- Statement compilation is over ---


            // now we can add the JumpIf instruction and the case statement instructions
            builder.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::JumpIf { 
                    jump_false: (case_statement.instructions.len() + 1) as i64,
                    unstack_len: 1, // unstack the boolean variable
                }
            });

            builder.extend(case_statement);

            if self.value.block_kind == WaitingBlockKind::First {
                success_jump_index.push(builder.instructions.len() - 1);
            }

            builder.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::Unstack { 
                    unstack_len: unstack_len_condition
                },
            });
            builder.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::Push(Literal::Bool(false)),
            });

            builder.instructions.push(Instruction {
                pos: Some(case.pos.clone()),
                control: InstructionType::LocalAssignment {
                    index: 0,
                    operator: BinaryAssignmentOperator::OrAssign,
                    unstack_len: 1,
                },
            });

        }

        let wait_index = builder.instructions.len();
        if !self.value.start_atomic {
            builder.instructions.push(Instruction {
                pos: Some(self.pos.clone()),
                control: InstructionType::AtomicEnd,
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
            for index in success_jump_index.iter() {
                builder.instructions[*index].control =
                    InstructionType::Jump((wait_index - index) as i64);
            }
        }

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
