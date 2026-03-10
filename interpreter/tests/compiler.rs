use std::collections::HashMap;

use althread::{
    ast::{
        statement::expression::{
            binary_expression::LocalBinaryExpressionNode,
            primary_expression::{LocalLiteralNode, LocalPrimaryExpressionNode, LocalVarNode},
            tuple_expression::LocalTupleExpressionNode,
            LocalExpressionNode,
        },
        token::{
            binary_assignment_operator::BinaryAssignmentOperator, binary_operator::BinaryOperator,
            literal::Literal,
        },
        Ast,
    },
    error::Pos,
    module_resolver::StandardFileSystem,
    vm::{instruction::{Instruction, InstructionType}, GlobalAction, VM},
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

    let compiled_project = ast
        .compile(std::path::Path::new(""), StandardFileSystem, &mut input_map)
        .unwrap();

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
fn test_condition_quantifiers_and_if_expr() {
    let input = r#"
shared {
    let Xs = [1..4];
    let Flag = true;
}

always {
    for x in Xs { x > 0 } && if Flag { exists y in Xs { y == 2 } } else { false };
}

main {
}
"#;

    let mut input_map = HashMap::new();
    input_map.insert("".to_string(), input.to_string());

    let pairs = althread::parser::parse(input, "").unwrap();
    let ast = Ast::build(pairs, "").unwrap();

    let compiled_project = ast
        .compile(std::path::Path::new(""), StandardFileSystem, &mut input_map)
        .unwrap();

    assert_eq!(compiled_project.always_conditions.len(), 1);

    let (vars, _, expr, _) = &compiled_project.always_conditions[0];

    assert!(vars.contains("Xs"));
    assert!(vars.contains("Flag"));
    assert!(!vars.contains("x"));
    assert!(!vars.contains("y"));

    let expr = match expr {
        LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(inner)) => {
            inner.as_ref()
        }
        _ => expr,
    };

    match expr {
        LocalExpressionNode::Binary(bin) => {
            assert_eq!(bin.operator, BinaryOperator::And);
            assert!(matches!(&*bin.left, LocalExpressionNode::ForAll(_)));
            match &*bin.right {
                LocalExpressionNode::IfExpr(if_node) => {
                    assert!(matches!(
                        &*if_node.condition,
                        LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Var(_))
                    ));
                    assert!(matches!(
                        &*if_node.then_expr,
                        LocalExpressionNode::Exists(_)
                    ));
                    assert!(matches!(
                        if_node.else_expr.as_ref().unwrap().as_ref(),
                        LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Literal(_))
                    ));
                }
                _ => panic!("Expected if-expression on right side of '&&'"),
            }
        }
        _ => panic!("Expected binary '&&' expression"),
    }
}

#[test]
fn test_wait_first_can_match_later_receive_case() {
    let input = r#"
program A() {
    await first {
        receive chin (msg) => {
            print("reçu:", msg);
        }
        receive chin2 (msg) => {
            print("reçu:", msg);
        }
    }
}

program B() {
    send chout("hello from B");
}

main {
    let a = run A();
    let b = run B();
    channel self.chout (string)> a.chin;
    channel b.chout (string)> a.chin2;
    send chout("hello from main");
}
"#;

    let mut input_map = HashMap::new();
    input_map.insert("".to_string(), input.to_string());

    let pairs = althread::parser::parse(input, "").unwrap();
    let ast = Ast::build(pairs, "").unwrap();
    let compiled_project = ast
        .compile(std::path::Path::new(""), StandardFileSystem, &mut input_map)
        .unwrap();

    let mut vm = VM::new(&compiled_project);
    vm.start(0);

    let (_, _, _, _, after_main) = vm.next().unwrap().into_iter().next().unwrap();
    let (_, _, _, _, after_b) = after_main
        .next()
        .unwrap()
        .into_iter()
        .find(|(name, pid, _, _, _)| name == "B" && *pid == 2)
        .unwrap();

    let (_, _, _, _, after_b_delivery) = after_b
        .next()
        .unwrap()
        .into_iter()
        .find(|(name, _, _, _, _)| name == "__deliver__ chin2#1")
        .unwrap();

    let next_states = after_b_delivery.next().unwrap();
    let a_step = next_states
        .iter()
        .find(|(name, pid, _, _, _)| name == "A" && *pid == 1)
        .expect("A should be schedulable after a message arrives on chin2");

    assert!(matches!(
        &a_step.4.get_program(1).current_instruction().unwrap().control,
        InstructionType::ChannelPeek(channel) if channel == "chin2"
    ));
}

#[test]
fn test_wait_first_eventually_consumes_later_receive_case() {
    let input = r#"
program A() {
    await first {
        receive chin (msg) => {
            print("reçu:", msg);
        }
        receive chin2 (msg) => {
            print("reçu:", msg);
        }
    }
}

program B() {
    send chout("hello from B");
}

main {
    let a = run A();
    let b = run B();
    channel self.chout (string)> a.chin;
    channel b.chout (string)> a.chin2;
    send chout("hello from main");
}
"#;

    let mut input_map = HashMap::new();
    input_map.insert("".to_string(), input.to_string());

    let pairs = althread::parser::parse(input, "").unwrap();
    let ast = Ast::build(pairs, "").unwrap();
    let compiled_project = ast
        .compile(std::path::Path::new(""), StandardFileSystem, &mut input_map)
        .unwrap();

    let mut vm = VM::new(&compiled_project);
    vm.start(0);

    let (_, _, _, _, after_main) = vm.next().unwrap().into_iter().next().unwrap();
    let (_, _, _, _, after_b) = after_main
        .next()
        .unwrap()
        .into_iter()
        .find(|(name, pid, _, _, _)| name == "B" && *pid == 2)
        .unwrap();

    let (_, _, _, _, after_b_delivery) = after_b
        .next()
        .unwrap()
        .into_iter()
        .find(|(name, _, _, _, _)| name == "__deliver__ chin2#1")
        .unwrap();

    let (_, _, _, _, after_a_ready) = after_b_delivery
        .next()
        .unwrap()
        .into_iter()
        .find(|(name, pid, _, _, _)| name == "A" && *pid == 1)
        .unwrap();

    let (_, _, _, actions, _) = after_a_ready
        .next()
        .unwrap()
        .into_iter()
        .find(|(name, pid, _, _, _)| name == "A" && *pid == 1)
        .unwrap();

    assert!(actions.iter().any(|action| matches!(
        action,
        GlobalAction::Print(msg) if msg == "reçu: hello from B"
    )));
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
                end: 39,
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
                end: 39,
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
                end: 78,
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
                end: 78,
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
                        LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                            value: Literal::String("done".to_string()),
                        }),
                    )],
                },
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

    let compiled_project = ast
        .compile(std::path::Path::new(""), StandardFileSystem, &mut input_map)
        .unwrap();

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
fn test_int_declaration() {
    let input = r#"
main {
    let a = 5;
    let b = 0X2a;
    let c = 0x2A;
    let d = 0b1010;
    let e = 0B1010;
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
                end: 39,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Primary(
                LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                    value: Literal::Int(42),
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
                start: 53,
                end: 57,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Primary(
                LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                    value: Literal::Int(42),
                }),
            )),
        },
        Instruction {
            pos: Some(Pos {
                line: 5,
                col: 5,
                start: 45,
                end: 48,
                file_path: "".to_string(),
            }),
            control: InstructionType::Declaration { unstack_len: 1 },
        },
        Instruction {
            pos: Some(Pos {
                line: 6,
                col: 13,
                start: 71,
                end: 77,
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
                line: 6,
                col: 5,
                start: 63,
                end: 66,
                file_path: "".to_string(),
            }),
            control: InstructionType::Declaration { unstack_len: 1 },
        },
        Instruction {
            pos: Some(Pos {
                line: 7,
                col: 13,
                start: 91,
                end: 97,
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
                line: 7,
                col: 5,
                start: 83,
                end: 86,
                file_path: "".to_string(),
            }),
            control: InstructionType::Declaration { unstack_len: 1 },
        },
        Instruction {
            pos: None,
            control: InstructionType::Unstack { unstack_len: 5 },
        },
        Instruction {
            pos: Some(Pos {
                line: 2,
                col: 6,
                start: 6,
                end: 100,
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

    let compiled_project = ast
        .compile(std::path::Path::new(""), StandardFileSystem, &mut input_map)
        .unwrap();

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
fn test_shift_operators() {
    let input = r#"
main {
    let a = 5 << 2;
    let b = 20 >> 1;
}
    "#;
    let expected = vec![
        Instruction {
            pos: Some(Pos {
                line: 3,
                col: 13,
                start: 20,
                end: 26,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Binary(
                LocalBinaryExpressionNode {
                    left: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                            value: Literal::Int(5),
                        }),
                    )),
                    operator: BinaryOperator::ShiftLeft,
                    right: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                            value: Literal::Int(2),
                        }),
                    )),
                },
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
                start: 40,
                end: 47,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Binary(
                LocalBinaryExpressionNode {
                    left: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                            value: Literal::Int(20),
                        }),
                    )),
                    operator: BinaryOperator::ShiftRight,
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
                line: 4,
                col: 5,
                start: 32,
                end: 35,
                file_path: "".to_string(),
            }),
            control: InstructionType::Declaration { unstack_len: 1 },
        },
        Instruction {
            pos: None,
            control: InstructionType::Unstack { unstack_len: 2 },
        },
        Instruction {
            pos: Some(Pos {
                line: 2,
                col: 6,
                start: 6,
                end: 50,
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

    let compiled_project = ast
        .compile(std::path::Path::new(""), StandardFileSystem, &mut input_map)
        .unwrap();

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
fn test_bitwise_operators() {
    let input = r#"
main {
    let a = 5 & 3;
    let b = 5 | 2;
    let c = 5 & 3 | 2;
}
    "#;
    let expected = vec![
        Instruction {
            pos: Some(Pos {
                line: 3,
                col: 13,
                start: 20,
                end: 25,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Binary(
                LocalBinaryExpressionNode {
                    left: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                            value: Literal::Int(5),
                        }),
                    )),
                    operator: BinaryOperator::BitAnd,
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
                start: 39,
                end: 44,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Binary(
                LocalBinaryExpressionNode {
                    left: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                            value: Literal::Int(5),
                        }),
                    )),
                    operator: BinaryOperator::BitOr,
                    right: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                            value: Literal::Int(2),
                        }),
                    )),
                },
            )),
        },
        Instruction {
            pos: Some(Pos {
                line: 4,
                col: 5,
                start: 31,
                end: 34,
                file_path: "".to_string(),
            }),
            control: InstructionType::Declaration { unstack_len: 1 },
        },
        Instruction {
            pos: Some(Pos {
                line: 5,
                col: 13,
                start: 58,
                end: 67,
                file_path: "".to_string(),
            }),
            control: InstructionType::Expression(LocalExpressionNode::Binary(
                LocalBinaryExpressionNode {
                    left: Box::new(LocalExpressionNode::Binary(LocalBinaryExpressionNode {
                        left: Box::new(LocalExpressionNode::Primary(
                            LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                                value: Literal::Int(5),
                            }),
                        )),
                        operator: BinaryOperator::BitAnd,
                        right: Box::new(LocalExpressionNode::Primary(
                            LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                                value: Literal::Int(3),
                            }),
                        )),
                    })),
                    operator: BinaryOperator::BitOr,
                    right: Box::new(LocalExpressionNode::Primary(
                        LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
                            value: Literal::Int(2),
                        }),
                    )),
                },
            )),
        },
        Instruction {
            pos: Some(Pos {
                line: 5,
                col: 5,
                start: 50,
                end: 53,
                file_path: "".to_string(),
            }),
            control: InstructionType::Declaration { unstack_len: 1 },
        },
        Instruction {
            pos: None,
            control: InstructionType::Unstack { unstack_len: 3 },
        },
        Instruction {
            pos: Some(Pos {
                line: 2,
                col: 6,
                start: 6,
                end: 70,
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

    let compiled_project = ast
        .compile(std::path::Path::new(""), StandardFileSystem, &mut input_map)
        .unwrap();

    assert_eq!(
        compiled_project
            .programs_code
            .get("main")
            .unwrap()
            .instructions,
        expected
    );
}