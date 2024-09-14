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
    compiler::{CompilerState, Variable},
    error::AlthreadResult,
    no_rule,
    parser::Rule,
    vm::instruction::{
        Instruction, InstructionType, JumpControl, JumpIfControl, LocalAssignmentControl,
        WaitControl, WaitStartControl,
    },
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
}

impl NodeBuilder for Wait {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
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
                    let node: Node<WaitingBlockCase> = Node::build(sub_pair)?;
                    children.push(node);
                }
                children
            }
            Rule::waiting_block_case => {
                let node: Node<WaitingBlockCase> = Node::build(pair)?;
                vec![node]
            }
            _ => {
                return Err(no_rule!(pair, "Wait"));
            }
        };

        Ok(Self {
            block_kind,
            waiting_cases,
        })
    }
}

impl InstructionBuilder for Node<Wait> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {
        let mut instructions = Vec::new();

        let mut dependencies = WaitDependency::new();
        for wc in self.value.waiting_cases.iter() {
            wc.value.rule.add_dependencies(&mut dependencies);
        }

        instructions.push(Instruction {
            pos: Some(self.pos),
            control: InstructionType::WaitStart(WaitStartControl { dependencies }),
        });

        state.program_stack.push(Variable {
            datatype: DataType::Boolean,
            name: "".to_string(),
            mutable: true,
            depth: state.current_stack_depth,
            declare_pos: None,
        });

        instructions.push(Instruction {
            pos: Some(self.pos),
            control: InstructionType::Push(Literal::Bool(false)),
        });

        let jump_if_offset = if self.value.block_kind == WaitingBlockKind::First {
            1
        } else {
            0
        };
        let mut jump_index = Vec::new();
        for case in &self.value.waiting_cases {
            state.current_stack_depth += 1;
            let mut case_condition = case.value.rule.compile(state)?;
            let unstack_len = state.unstack_current_depth();

            let mut case_statement = match &case.value.statement {
                Some(s) => s.compile(state)?,
                None => vec![],
            };

            case_statement.push(Instruction {
                pos: Some(case.pos),
                control: InstructionType::Push(Literal::Bool(true)),
            });
            case_statement.push(Instruction {
                pos: Some(case.pos),
                control: InstructionType::LocalAssignment(LocalAssignmentControl {
                    index: 0,
                    operator: BinaryAssignmentOperator::OrAssign,
                    unstack_len: 1,
                }),
            });

            // the offset is because a jump will be added after the statement
            case_condition.push(Instruction {
                pos: Some(case.pos),
                control: InstructionType::JumpIf(JumpIfControl {
                    jump_false: (case_statement.len() + 1 + jump_if_offset) as i64,
                    unstack_len,
                }),
            });
            instructions.extend(case_condition);
            instructions.extend(case_statement);
            jump_index.push(instructions.len());
        }

        if self.value.block_kind == WaitingBlockKind::First {
            for index in jump_index.iter().rev() {
                instructions.insert(
                    *index,
                    Instruction {
                        pos: Some(self.pos),
                        control: InstructionType::Jump(JumpControl {
                            jump: (instructions.len() - index + 1) as i64,
                        }),
                    },
                );
            }
        }

        instructions.push(Instruction {
            pos: Some(self.pos),
            control: InstructionType::Wait(WaitControl {
                jump: -(instructions.len() as i64),
                unstack_len: 1,
            }),
        });
        state.program_stack.pop();
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

        Ok(instructions)
    }
}

impl AstDisplay for Wait {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}wait_control")?;
        {
            for case in &self.waiting_cases {
                case.ast_fmt(f, &prefix.add_branch())?;
            }
        }

        Ok(())
    }
}
