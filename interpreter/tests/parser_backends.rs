use althread::{
    ast::statement::expression::primary_expression::PrimaryExpression,
    ast::statement::{
        expression::{CallChainSegment, Expression},
        Statement,
    },
    parser::{chumsky_combinator, parse_ast, syntax::SyntaxSnippet},
};

#[test]
fn parse_ast_builds_shared_block_directly() {
    let source = r#"
shared {
    let Count: int = 1;
    const Names: list(string) = ["a", "b"];
}
"#;

    let ast = parse_ast(source, "").unwrap();
    let shared = ast.global_block.expect("shared block should be present");

    assert_eq!(shared.value.children.len(), 2);
    assert!(matches!(
        &shared.value.children[0].value,
        Statement::Declaration(_)
    ));
    assert!(matches!(
        &shared.value.children[1].value,
        Statement::Declaration(_)
    ));
}

#[test]
fn parse_ast_builds_import_block_directly() {
    let source = r#"
import {
    std/io;
    math/vector as Vec;
}

main {
}
"#;

    let ast = parse_ast(source, "").unwrap();
    let import_block = ast.import_block.expect("import block should be present");

    assert_eq!(import_block.value.imports.len(), 2);
    assert_eq!(
        import_block.value.imports[0].value.path.segments,
        vec!["std", "io"]
    );
    assert!(import_block.value.imports[0].value.alias.is_none());
    assert_eq!(
        import_block.value.imports[1]
            .value
            .alias
            .as_ref()
            .expect("alias should exist")
            .value
            .value,
        "Vec"
    );
}

#[test]
fn combinator_parser_exposes_shared_ast_shape() {
    let source = r#"
shared {
    let Count: int = 1;
}
"#;

    let ast = chumsky_combinator::parse_program(source, "").unwrap();
    let shared = ast.global_block.expect("shared block should be present");

    assert_eq!(shared.value.children.len(), 1);
    match &shared.value.children[0].value {
        Statement::Declaration(node) => {
            assert_eq!(node.value.identifier.value.parts.len(), 1);
            assert_eq!(node.value.identifier.value.parts[0].value.value, "Count");
        }
        other => panic!("expected declaration, got {other:?}"),
    }
}

#[test]
fn parse_ast_builds_main_block_directly() {
    let source = r#"
main {
    let x = 1 + 2;
    print(x);
}
"#;

    let ast = parse_ast(source, "").unwrap();
    let (_, main_block, _) = ast
        .process_blocks
        .get("main")
        .expect("main block should exist");

    assert_eq!(main_block.value.children.len(), 2);
    assert!(matches!(
        &main_block.value.children[0].value,
        Statement::Declaration(_)
    ));
    assert!(matches!(
        &main_block.value.children[1].value,
        Statement::FnCall(_)
    ));
}

#[test]
fn parse_ast_builds_program_blocks_with_args() {
    let source = r#"
@private program Worker(id: int, name: string) {
    let pid = run Child(id);
}
"#;

    let ast = parse_ast(source, "").unwrap();
    let (args, block, is_private) = ast
        .process_blocks
        .get("Worker")
        .expect("program block should exist");

    assert!(*is_private);
    assert_eq!(args.value.identifiers.len(), 2);
    assert_eq!(args.value.identifiers[0].value.value, "id");
    assert_eq!(args.value.identifiers[1].value.value, "name");
    assert_eq!(block.value.children.len(), 1);
    assert!(matches!(
        &block.value.children[0].value,
        Statement::Declaration(_)
    ));
}

#[test]
fn parse_ast_builds_function_blocks_with_return_type() {
    let source = r#"
@private fn add(a: int, b: int) -> int {
    return a + b;
}

fn nop() -> void {
    return;
}
"#;

    let ast = parse_ast(source, "").unwrap();

    let (args, return_type, block, is_private) = ast
        .function_blocks
        .get("add")
        .expect("function add should exist");
    assert!(*is_private);
    assert_eq!(args.value.identifiers.len(), 2);
    assert_eq!(return_type.to_string(), "int");
    assert_eq!(block.value.children.len(), 1);
    assert!(matches!(
        &block.value.children[0].value,
        Statement::FnReturn(_)
    ));

    let (_, return_type, block, is_private) = ast
        .function_blocks
        .get("nop")
        .expect("function nop should exist");
    assert!(!*is_private);
    assert_eq!(return_type.to_string(), "void");
    assert!(matches!(
        &block.value.children[0].value,
        Statement::FnReturn(_)
    ));
}

#[test]
fn expression_parser_accepts_run_calls() {
    let source = "run Worker(1 + 2)";
    let snippet = SyntaxSnippet::new(
        althread::error::Pos::from_offsets(source, "", 0, source.len()),
        source.to_string(),
    );

    let expr = chumsky_combinator::parse_expression(source, &snippet, "").unwrap();
    assert!(matches!(expr.value, Expression::RunCall(_)));
}

#[test]
fn parse_ast_accepts_basic_control_flow_in_main() {
    let source = r#"
main {
    let xs = [1, 2, 3];
    for x in xs {
        if x == 1 {
            print("one");
        } else if x == 2 {
            print("two");
        } else {
            print("other");
        }
    }

    while true {
        break;
    }

    loop {
        continue;
    }

    atomic {
        print("atomic");
    }

    @ {
        print("shorthand");
    }
}
"#;

    let ast = parse_ast(source, "").unwrap();
    let (_, main_block, _) = ast
        .process_blocks
        .get("main")
        .expect("main block should exist");

    assert!(main_block
        .value
        .children
        .iter()
        .any(|child| matches!(child.value, Statement::For(_))));
    assert!(main_block
        .value
        .children
        .iter()
        .any(|child| matches!(child.value, Statement::While(_))));
    assert!(main_block
        .value
        .children
        .iter()
        .any(|child| matches!(child.value, Statement::Loop(_))));
    assert!(
        main_block
            .value
            .children
            .iter()
            .filter(|child| matches!(child.value, Statement::Atomic(_)))
            .count()
            >= 2
    );
}

#[test]
fn parse_ast_accepts_await_statements_and_chaining() {
    let source = r#"
shared {
    let Value = 0;
}

program A() {
    loop atomic await receive in(x) => {
        if x == 1 {
            break;
        }
    }
}

main {
    await Value == 1;
    await receive inbox(msg);
    await first {
        receive inbox(msg) => print(msg);
        Value == 2 => {
            print("two");
        }
    }
}
"#;

    let ast = parse_ast(source, "").unwrap();
    let (_, program_block, _) = ast.process_blocks.get("A").expect("program A should exist");
    let (_, main_block, _) = ast
        .process_blocks
        .get("main")
        .expect("main block should exist");

    assert!(matches!(
        program_block.value.children[0].value,
        Statement::Loop(_)
    ));
    assert!(
        main_block
            .value
            .children
            .iter()
            .filter(|child| matches!(child.value, Statement::Wait(_)))
            .count()
            >= 3
    );
}

#[test]
fn parse_ast_accepts_send_and_channel_statements() {
    let source = r#"
program Worker() {
    send out(1, true);
    send out.a.*(42);
}

main {
    let a = run Worker();
    channel self.out (int, bool)> a.in;
    channel a.out.a.a (int)> self.in;
}
"#;

    let ast = parse_ast(source, "").unwrap();
    let (_, program_block, _) = ast
        .process_blocks
        .get("Worker")
        .expect("program Worker should exist");
    let (_, main_block, _) = ast
        .process_blocks
        .get("main")
        .expect("main block should exist");

    assert!(matches!(
        program_block.value.children[0].value,
        Statement::Send(_)
    ));
    assert!(matches!(
        program_block.value.children[1].value,
        Statement::Send(_)
    ));
    assert!(matches!(
        main_block.value.children[1].value,
        Statement::ChannelDeclaration(_)
    ));
    assert!(matches!(
        main_block.value.children[2].value,
        Statement::ChannelDeclaration(_)
    ));
}

#[test]
fn parse_ast_accepts_first_and_seq_as_identifiers() {
    let source = r#"
main {
    let first = p.at(0);
    let seq = p.at(1);
}
"#;

    let ast = parse_ast(source, "").unwrap();
    let (_, main_block, _) = ast
        .process_blocks
        .get("main")
        .expect("main block should exist");

    assert!(matches!(
        &main_block.value.children[0].value,
        Statement::Declaration(_)
    ));
    assert!(matches!(
        &main_block.value.children[1].value,
        Statement::Declaration(_)
    ));
}

#[test]
fn parse_ast_ignores_multiline_comments() {
    let source = r#"
/* top level comment
   with fake syntax: await first { receive in(x) => run A(); }
*/
main {
    /* declaration comment */
    let x = 1;
    /*
      another comment
    */
    print(x);
}
"#;

    let ast = parse_ast(source, "").unwrap();
    let (_, main_block, _) = ast
        .process_blocks
        .get("main")
        .expect("main block should exist");

    assert_eq!(main_block.value.children.len(), 2);
    assert!(matches!(
        &main_block.value.children[0].value,
        Statement::Declaration(_)
    ));
    assert!(matches!(
        &main_block.value.children[1].value,
        Statement::FnCall(_)
    ));
}

#[test]
fn parse_ast_accepts_always_blocks() {
    let source = r#"
shared {
    let Xs = 1..4;
    let Flag = true;
}

always {
    for x in Xs { x > 0 } && if Flag { exists y in Xs { y == 2 } } else { false };
    Xs.len() > 0;
}

main {
}
"#;

    let ast = parse_ast(source, "").unwrap();
    let always = ast
        .condition_blocks
        .get(&althread::ast::token::condition_keyword::ConditionKeyword::Always)
        .expect("always block should exist");

    assert_eq!(always.value.children.len(), 2);
}

#[test]
fn parse_ast_accepts_check_blocks() {
    let source = r#"
shared {
    let Request = false;
    let Granted = false;
}

check {
    always (if Request { eventually Granted });
    eventually (Granted == true);
}

main {
}
"#;

    let ast = parse_ast(source, "").unwrap();

    assert_eq!(ast.check_blocks.len(), 1);
    assert_eq!(ast.check_blocks[0].value.formulas.len(), 2);
}

#[test]
fn expression_parser_accepts_arithmetic() {
    let snippet = SyntaxSnippet::new(
        althread::error::Pos::from_offsets("1 + 2 * 3", "", 0, "1 + 2 * 3".len()),
        "1 + 2 * 3".to_string(),
    );

    chumsky_combinator::parse_expression("1 + 2 * 3", &snippet, "").unwrap();
}

#[test]
fn expression_parser_accepts_function_calls_and_callchains() {
    let source = "worker.test(1).next(2)";
    let snippet = SyntaxSnippet::new(
        althread::error::Pos::from_offsets(source, "", 0, source.len()),
        source.to_string(),
    );

    chumsky_combinator::parse_expression(source, &snippet, "").unwrap();
}

#[test]
fn expression_parser_accepts_fields_and_tuple_indexes() {
    let source = "C.f().c.d.g()";
    let snippet = SyntaxSnippet::new(
        althread::error::Pos::from_offsets(source, "", 0, source.len()),
        source.to_string(),
    );

    let expr = chumsky_combinator::parse_expression(source, &snippet, "").unwrap();

    let Expression::CallChain(node) = &expr.value else {
        panic!("expected a call chain, got {:?}", expr.value);
    };

    assert!(matches!(
        node.value.segments.last(),
        Some(CallChainSegment::Call { .. } | CallChainSegment::Invoke { .. })
    ));
    assert_eq!(
        node.value
            .segments
            .iter()
            .filter(|segment| matches!(segment, CallChainSegment::Field { .. }))
            .count(),
        2
    );
}

#[test]
fn expression_parser_accepts_tuple_field_style_indexes() {
    let source = "a.0";
    let snippet = SyntaxSnippet::new(
        althread::error::Pos::from_offsets(source, "", 0, source.len()),
        source.to_string(),
    );

    let expr = chumsky_combinator::parse_expression(source, &snippet, "").unwrap();

    let Expression::CallChain(node) = &expr.value else {
        panic!("expected a call chain, got {:?}", expr.value);
    };

    assert!(matches!(
        node.value.segments.as_slice(),
        [CallChainSegment::TupleIndex { index: 0 }]
    ));
}

#[test]
fn expression_parser_keeps_only_single_identifier_in_chain_base() {
    let source = "a.b.c()";
    let snippet = SyntaxSnippet::new(
        althread::error::Pos::from_offsets(source, "", 0, source.len()),
        source.to_string(),
    );

    let expr = chumsky_combinator::parse_expression(source, &snippet, "").unwrap();

    let Expression::CallChain(node) = &expr.value else {
        panic!("expected a call chain, got {:?}", expr.value);
    };

    let Expression::Primary(primary) = &node.value.base.value else {
        panic!("expected a primary base, got {:?}", node.value.base.value);
    };

    let PrimaryExpression::Identifier(identifier) = &primary.value else {
        panic!("expected an identifier base, got {:?}", primary.value);
    };

    assert_eq!(identifier.value.parts.len(), 1);
    assert_eq!(identifier.value.parts[0].value.value, "a");
    assert!(matches!(
        node.value.segments[0],
        CallChainSegment::Field { .. }
    ));
}

#[test]
fn expression_parser_accepts_simple_ranges() {
    let source = "0..5";
    let snippet = SyntaxSnippet::new(
        althread::error::Pos::from_offsets(source, "", 0, source.len()),
        source.to_string(),
    );

    let expr = chumsky_combinator::parse_expression(source, &snippet, "").unwrap();
    assert!(matches!(expr.value, Expression::Range(_)));
}

#[test]
fn expression_parser_accepts_ranges_with_arithmetic_bounds() {
    let source = "0..(N * 3)";
    let snippet = SyntaxSnippet::new(
        althread::error::Pos::from_offsets(source, "", 0, source.len()),
        source.to_string(),
    );

    let expr = chumsky_combinator::parse_expression(source, &snippet, "").unwrap();
    assert!(matches!(expr.value, Expression::Range(_)));
}

#[test]
fn expression_parser_accepts_ranges_inside_tuples() {
    let source = "(1, 4..10)";
    let snippet = SyntaxSnippet::new(
        althread::error::Pos::from_offsets(source, "", 0, source.len()),
        source.to_string(),
    );

    let expr = chumsky_combinator::parse_expression(source, &snippet, "").unwrap();
    assert!(matches!(expr.value, Expression::Tuple(_)));
}

#[test]
fn datatype_errors_are_human_readable() {
    let source = r#"
shared {
    let a:;
}
"#;

    let err = parse_ast(source, "").unwrap_err();

    assert!(err.message.contains("expected a datatype"));
    assert!(err.message.contains("found ';'"));
}
