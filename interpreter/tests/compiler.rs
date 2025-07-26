use std::{collections::HashMap};

use althread::{
    ast::{
        statement::expression::{
            binary_expression::LocalBinaryExpressionNode, primary_expression::{LocalLiteralNode, LocalPrimaryExpressionNode, LocalVarNode}, tuple_expression::LocalTupleExpressionNode, LocalExpressionNode
        },
        token::{
            binary_assignment_operator::BinaryAssignmentOperator, binary_operator::BinaryOperator,
            literal::Literal,
        },
        Ast,
    }, error::Pos, module_resolver::StandardFileSystem, vm::instruction::{Instruction, InstructionType}
};

// A simple test to verify that the compiler can compile a simple program
#[test]
fn test_compiler_expression() {
    let input = r#"
main {
    let x = 5;
    let y = 10;
    let z = 15;
    let result = x + y + z;
}
    "#;

    let expected = vec![
        Instruction {
            pos: Some(Pos {
                line: 3,
                col: 13,
                start: 20,
                end: 21,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Primary(
                LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                    value: Literal::Int(5),
                }),
            )),
        },
        Instruction {
            pos: Some(Pos {
                line: 3,
                col: 5,
                start: 12,
                end: 15,
                file_path: "".to_string(),
            }),
            control: InstructionType::Declaration { unstack_len: 1 },
        },
        Instruction {
            pos: Some(Pos {
                line: 4,
                col: 13,
                start: 35,
                end: 37,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Primary(
                LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                    value: Literal::Int(10),
                }),
            )),
        },
        Instruction {
            pos: Some(Pos {
                line: 4,
                col: 5,
                start: 27,
                end: 30,
                file_path: "".to_string(),
            }),
            control: InstructionType::Declaration { unstack_len: 1 },
        },
        Instruction {
            pos: Some(Pos {
                line: 5,
                col: 13,
                start: 51,
                end: 53,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Primary(
                LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                    value: Literal::Int(15),
                }),
            )),
        },
        Instruction {
            pos: Some(Pos {
                line: 5,
                col: 5,
                start: 43,
                end: 46,
                file_path: "".to_string(),
            }),
            control: InstructionType::Declaration { unstack_len: 1 },
        },
        Instruction {
            pos: Some(Pos {
                line: 6,
                col: 18,
                start: 72,
                end: 81,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Binary(
                LocalBinaryExpressionNode {
                    left: Box::new(LocalExpressionNode::Binary(LocalBinaryExpressionNode {
                        left: Box::new(LocalExpressionNode::Primary(
                            LocalPrimaryExpressionNode::Var(LocalVarNode { index: 2 }),
                        )),
                        operator: BinaryOperator::Add,
                        right: Box::new(LocalExpressionNode::Primary(
                            LocalPrimaryExpressionNode::Var(LocalVarNode { index: 1 }),
                        )),
                    })),
                    operator: BinaryOperator::Add,
                    right: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Var(LocalVarNode { index: 0 }),
                    )),
                },
            )),
        },
        Instruction {
            pos: Some(Pos {
                line: 6,
                col: 5,
                start: 59,
                end: 62,
                file_path: "".to_string(),
            }),
            control: InstructionType::Declaration { unstack_len: 1 },
        },
        Instruction {
            pos: None,
            control: InstructionType::Unstack { unstack_len: 4 },
        },
        Instruction {
            pos: Some(Pos {
                line: 2,
                col: 6,
                start: 6,
                end: 84,
                file_path: "".to_string(),
            }),
            control: InstructionType::EndProgram,
        },
    ];

    let mut input_map = HashMap::new();
    input_map.insert("".to_string(), input.to_string());

    // parse code with pest
    let pairs = althread::parser::parse(input, "").unwrap();

    let ast = Ast::build(pairs, "").unwrap();

    let compiled_project = ast.compile(std::path::Path::new(""), StandardFileSystem, &mut input_map).unwrap();

    assert_eq!(
        compiled_project
            .programs_code
            .get("main")
            .unwrap()
            .instructions,
        expected
    );
}

#[test]
fn test_compiler_while() {
    let input = r#"
main {
    let x = 0;
    while x < 5 {
        x = x + 1;
        if x == 3 {
            break;
        }
    }
    print("done");
}
"#;

    let expected = vec![
        Instruction {
            pos: Some(Pos {
                line: 3,
                col: 13,
                start: 20,
                end: 21,
            file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Primary(
                LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                    value: Literal::Int(0),
                }),
            )),
        },
        Instruction {
            pos: Some(Pos {
                line: 3,
                col: 5,
                start: 12,
                end: 15,
                file_path: "".to_string(),
            }),
            control: InstructionType::Declaration { unstack_len: 1 },
        },
        Instruction {
            pos: Some(Pos {
                line: 4,
                col: 11,
                start: 33,
                end: 38,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Binary(
                LocalBinaryExpressionNode {
                    left: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Var(LocalVarNode { index: 0 }),
                    )),
                    operator: BinaryOperator::LessThan,
                    right: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                            value: Literal::Int(5),
                        }),
                    )),
                },
            )),
        },
        Instruction {
            pos: Some(Pos {
                line: 4,
                col: 11,
                start: 33,
                end: 38,
                file_path: "".to_string(),
            }),
            control: InstructionType::JumpIf {
                jump_false: 8,
                unstack_len: 1,
            },
        },
        Instruction {
            pos: Some(Pos {
                line: 5,
                col: 13,
                start: 53,
                end: 58,
                file_path: "".to_string(),  
            }),
            control: InstructionType::Expression(LocalExpressionNode::Binary(
                LocalBinaryExpressionNode {
                    left: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Var(LocalVarNode { index: 0 }),
                    )),
                    operator: BinaryOperator::Add,
                    right: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                            value: Literal::Int(1),
                        }),
                    )),
                },
            )),
        },
        Instruction {
            pos: Some(Pos {
                line: 5,
                col: 9,
                start: 49,
                end: 51,
                file_path: "".to_string(),
            }),
            control: InstructionType::LocalAssignment {
                index: 0,
                operator: BinaryAssignmentOperator::Assign,
                unstack_len: 1,
            },
        },
        Instruction {
            pos: Some(Pos {
                line: 6,
                col: 12,
                start: 71,
                end: 77,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Binary(
                LocalBinaryExpressionNode {
                    left: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Var(LocalVarNode { index: 0 }),
                    )),
                    operator: BinaryOperator::Equals,
                    right: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                            value: Literal::Int(3),
                        }),
                    )),
                },
            )),
        },
        Instruction {
            pos: Some(Pos {
                line: 6,
                col: 12,
                start: 71,
                end: 77,
                file_path: "".to_string(),
            }),
            control: InstructionType::JumpIf {
                jump_false: 3,
                unstack_len: 1,
            },
        },
        Instruction {
            pos: None,
            control: InstructionType::Break {
                jump: 3,
                unstack_len: 0,
                stop_atomic: false,
            },
        },
        Instruction {
            pos: Some(Pos {
                line: 6,
                col: 19,
                start: 78,
                end: 108,
                file_path: "".to_string(),
            }),
            control: InstructionType::Empty,
        },
        Instruction {
            pos: Some(Pos {
                line: 4,
                col: 5,
                start: 27,
                end: 114,
                file_path: "".to_string(),
            }),
            control: InstructionType::Jump(-8),
        },
        Instruction {
            pos: Some(Pos {
                line: 10,
                col: 10,
                start: 124,
                end: 132,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Tuple(
                LocalTupleExpressionNode {
                    values: vec![LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Literal(
                            LocalLiteralNode {
                                value: Literal::String("done".to_string())
                            }
                        )
                    )],
                }
            )),
        },
        Instruction {
            pos: Some(Pos {
                line: 10,
                col: 5,
                start: 119,
                end: 132,
                file_path: "".to_string(),
            }),
            control: InstructionType::FnCall {
                name: "print".to_string(),
                unstack_len: 1,
                variable_idx: None,
                arguments: None,
            },
        },
        Instruction {
            pos: Some(Pos {
                line: 10,
                col: 5,
                start: 119,
                end: 132,
                file_path: "".to_string(),
            }),
            control: InstructionType::Unstack { unstack_len: 1 },
        },
        Instruction {
            pos: None,
            control: InstructionType::Unstack { unstack_len: 1 },
        },
        Instruction {
            pos: Some(Pos {
                line: 2,
                col: 6,
                start: 6,
                end: 135,
                file_path: "".to_string(),
            }),
            control: InstructionType::EndProgram,
        },
    ];

    let mut input_map = HashMap::new();
    input_map.insert("".to_string(), input.to_string());

    // parse code with pest
    let pairs = althread::parser::parse(input, "").unwrap();

    let ast = Ast::build(pairs, "").unwrap();

    let compiled_project = ast.compile(std::path::Path::new(""), StandardFileSystem, &mut input_map).unwrap();

    assert_eq!(
        compiled_project
            .programs_code
            .get("main")
            .unwrap()
            .instructions,
        expected
    );
}
