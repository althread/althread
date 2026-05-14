use althread::{
    ast::statement::{
        expression::{CallChainSegment, Expression},
        Statement,
    },
    ast::statement::expression::primary_expression::PrimaryExpression,
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
fn unsupported_blocks_fail_for_now() {
    let source = r#"
main {
}
"#;

    assert!(parse_ast(source, "").is_err());
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
    assert!(matches!(node.value.segments[0], CallChainSegment::Field { .. }));
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
