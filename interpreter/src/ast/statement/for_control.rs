use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        statement::expression::{
            binary_expression::LocalBinaryExpressionNode,
            primary_expression::{LocalPrimaryExpressionNode, LocalVarNode},
            LocalExpressionNode,
        },
        token::{
            binary_assignment_operator::BinaryAssignmentOperator, binary_operator::BinaryOperator,
            datatype::DataType, identifier::Identifier, literal::Literal,
        },
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType},
    no_rule,
    parser::Rule,
    vm::instruction::{
        ExpressionControl, FnCallControl, Instruction, InstructionType, JumpControl, JumpIfControl,
        LocalAssignmentControl, UnstackControl,
    },
};

use super::{
    expression::Expression,
    Statement,
};

#[derive(Debug, Clone)]
pub struct ForControl {
    pub identifier: Node<Identifier>,
    pub expression: Node<Expression>,
    pub statement: Box<Node<Statement>>,
}

impl NodeBuilder for ForControl {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let identifier = Node::build(pairs.next().unwrap())?;
        let exp_pair = pairs.next().unwrap();
        let expression = match exp_pair.as_rule() {
            Rule::expression => Node::build(exp_pair),
            Rule::range_expression => Expression::build_list_expression(exp_pair),
            _ => Err(no_rule!(exp_pair, "For loop expression")),
        }?;
        let statement = Box::new(Node::build(pairs.next().unwrap())?);

        Ok(Self {
            statement,
            identifier,
            expression,
        })
    }
}

impl InstructionBuilder for Node<ForControl> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let stack_len = state.program_stack.len();
        let mut builder = InstructionBuilderOk::new();

        state.current_stack_depth += 1;
        // compile the expression (this pushes the list to the stack)
        builder.extend(self.value.expression.compile(state)?);
        let dtype = &state
            .program_stack
            .last()
            .expect("stack should contain a value after an expression is compiled")
            .datatype;

        let list_type = match dtype {
            DataType::List(list_type) => list_type.as_ref().clone(),
            _ => {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos),
                    format!("The expression is not a list ({} is given)", dtype.clone()),
                ))
            }
        };
        // make sure the interface is built for this list:
        state
            .stdlib
            .interfaces(&DataType::List(Box::new(list_type.clone())));

        // push the iterator variable
        state.program_stack.push(Variable {
            name: self.value.identifier.value.value.clone(),
            datatype: list_type.clone(),
            declare_pos: Some(self.value.identifier.pos),
            depth: state.current_stack_depth,
            mutable: true,
        });
        builder.instructions.push(Instruction {
            pos: Some(self.value.identifier.pos),
            control: InstructionType::Push(list_type.default()),
        });
        // push the iterator index
        state.program_stack.push(Variable {
            name: "".to_string(),
            datatype: list_type.clone(),
            declare_pos: Some(self.value.identifier.pos),
            depth: state.current_stack_depth,
            mutable: true,
        });
        builder.instructions.push(Instruction {
            pos: Some(self.value.identifier.pos),
            control: InstructionType::Push(Literal::Int(-1)),
        });

        // add the instruction to increment the the index
        builder.instructions.push(Instruction {
            pos: Some(self.value.identifier.pos),
            control: InstructionType::Push(Literal::Int(1)),
        });
        builder.instructions.push(Instruction {
            pos: Some(self.value.identifier.pos),
            control: InstructionType::LocalAssignment(LocalAssignmentControl {
                index: 0,
                operator: BinaryAssignmentOperator::AddAssign,
                unstack_len: 1,
            }),
        });
        // add the instruction to check if the index is greater than the length of the list
        builder.instructions.push(Instruction {
            pos: Some(self.value.identifier.pos),
            control: InstructionType::FnCall(FnCallControl {
                name: "len".to_string(),
                unstack_len: 0,
                variable_idx: Some(2),
                arguments: Some(vec![0]), // if the arguments are scattered in the stack
            }),
        });
        builder.instructions.push(Instruction {
            pos: Some(self.value.identifier.pos),
            control: InstructionType::Expression(ExpressionControl {
                root: LocalExpressionNode::Binary(LocalBinaryExpressionNode {
                    // idx < len(list)
                    left: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Var(LocalVarNode { index: 1 }),
                    )),
                    operator: BinaryOperator::LessThan,
                    right: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Var(LocalVarNode { index: 0 }),
                    )),
                }),
            }),
        });
        builder.instructions.push(Instruction {
            pos: Some(self.value.identifier.pos),
            control: InstructionType::JumpIf(JumpIfControl {
                jump_false: 0,
                unstack_len: 2,
            }),
        });
        let jump_idx = builder.instructions.len() - 1;

        // add the instruction that the variable takes the value of the element in the list at the position of the index
        builder.instructions.push(Instruction {
            pos: Some(self.value.identifier.pos),
            control: InstructionType::FnCall(FnCallControl {
                name: "at".to_string(),
                unstack_len: 0,
                variable_idx: Some(2),
                arguments: Some(vec![0]), // if the arguments are scattered in the stack
            }),
        });
        builder.instructions.push(Instruction {
            pos: Some(self.value.identifier.pos),
            control: InstructionType::LocalAssignment(LocalAssignmentControl {
                index: 1,
                operator: BinaryAssignmentOperator::Assign,
                unstack_len: 1,
            }),
        });

        let statement_builder = self.value.statement.as_ref().compile(state)?;
        let statement_len = statement_builder.instructions.len();

        builder.extend(statement_builder);

        builder.instructions.push(Instruction {
            pos: Some(self.value.statement.as_ref().pos),
            control: InstructionType::Jump(JumpControl {
                jump: -(statement_len as i64) - 7,
            }),
        });

        builder.instructions[jump_idx].control = InstructionType::JumpIf(JumpIfControl {
            jump_false: statement_len as i64 + 4, // statement len plus the assignment of the iterator variable
            unstack_len: 2,
        });

        let unstack_len = state.unstack_current_depth();

        assert!(unstack_len == 3);

        // unstack the list, iterator variable and index
        builder.instructions.push(Instruction {
            pos: Some(self.value.statement.as_ref().pos),
            control: InstructionType::Unstack(UnstackControl { unstack_len }),
        });

        assert!(stack_len == state.program_stack.len());

        if builder.contains_jump() {
            for idx in builder.break_indexes.get("").unwrap_or(&Vec::new()) {
                let builder_len = builder.instructions.len();
                if let InstructionType::Break(bc) = &mut builder.instructions[*idx as usize].control
                {
                    bc.jump = (builder_len - idx) as i64;
                    bc.unstack_len = bc.unstack_len - stack_len;
                } else {
                    panic!("Expected Break instruction");
                }
            }
            builder.break_indexes.remove("");
            for idx in builder.continue_indexes.get("").unwrap_or(&Vec::new()) {
                if let InstructionType::Break(bc) = &mut builder.instructions[*idx as usize].control
                {
                    bc.jump = -(*idx as i64);
                    bc.unstack_len = bc.unstack_len - stack_len;
                } else {
                    panic!("Expected Break instruction");
                }
            }
            builder.continue_indexes.remove("");
        }
        Ok(builder)
    }
}

impl AstDisplay for ForControl {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}for")?;

        let prefix = prefix.switch();
        writeln!(f, "{prefix}do")?;
        {
            let prefix = prefix.add_leaf();
            self.statement.as_ref().ast_fmt(f, &prefix)?;
        }

        Ok(())
    }
}
