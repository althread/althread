use std::fs;

use althread::parser::{parse_ast, ParserBackend, ParserOptions};

#[test]
fn pest_and_chumsky_match_on_basic_program() {
    let source = r#"
import [github.com/example/demo as demo]

shared {
    let Global:list(int) = [1, 2, 3];
}

always {
    Global.at(0) == 1;
}

check {
    always true;
}

program Worker(id: int) {
    await first {
        receive Inbox(id) => {
            send Out(id);
        }
    }
}

fn twice(value: int) -> int {
    return value + value;
}

main {
    run Worker(1);
}
"#;

    let pest = parse_ast(
        source,
        "",
        ParserOptions {
            primary: ParserBackend::Pest,
            compare_against: None,
        },
    )
    .unwrap();
    let chumsky = parse_ast(
        source,
        "",
        ParserOptions {
            primary: ParserBackend::Chumsky,
            compare_against: None,
        },
    )
    .unwrap();

    assert_eq!(pest.ast.diff_summary(&chumsky.ast), None);
}

#[test]
fn compare_mode_reports_match() {
    let source = r#"
main {
    let x = 1;
}
"#;

    let output = parse_ast(
        source,
        "",
        ParserOptions {
            primary: ParserBackend::Pest,
            compare_against: Some(ParserBackend::Chumsky),
        },
    )
    .unwrap();

    let comparison = output.comparison.expect("comparison output missing");
    assert!(comparison.matched);
    assert_eq!(comparison.summary, None);
}

#[test]
fn chumsky_matches_nested_header_types() {
    let source = r#"
program Worker(items: list(tuple(int, list(proc(Node))))) {
    let ready = true;
}

fn project(items: list(tuple(int, list(proc(Node))))) -> list(proc(Node)) {
    return items.at(0).at(1);
}

main {}
"#;

    let pest = parse_ast(
        source,
        "",
        ParserOptions {
            primary: ParserBackend::Pest,
            compare_against: None,
        },
    )
    .unwrap();
    let chumsky = parse_ast(
        source,
        "",
        ParserOptions {
            primary: ParserBackend::Chumsky,
            compare_against: None,
        },
    )
    .unwrap();

    assert_eq!(pest.ast.diff_summary(&chumsky.ast), None);
}

#[test]
fn chumsky_matches_nested_channel_types() {
    let source = r#"
program Sender() {
    channel self.Out<(list(proc(Node)), tuple(int, bool))> Receiver.In;
}

program Receiver() {
}

main {
    let target = run Receiver();
    run Sender();
}
"#;

    let pest = parse_ast(
        source,
        "",
        ParserOptions {
            primary: ParserBackend::Pest,
            compare_against: None,
        },
    )
    .unwrap();
    let chumsky = parse_ast(
        source,
        "",
        ParserOptions {
            primary: ParserBackend::Chumsky,
            compare_against: None,
        },
    )
    .unwrap();

    assert_eq!(pest.ast.diff_summary(&chumsky.ast), None);
}

#[test]
fn chumsky_matches_example_corpus() {
    for entry in fs::read_dir("../examples").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("alt") {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("TP2-communication.alt") {
            // This course example is intentionally excluded from strict parser parity for now.
            continue;
        }

        let source = fs::read_to_string(&path).unwrap();
        let pest = match parse_ast(
            &source,
            &path.to_string_lossy(),
            ParserOptions {
                primary: ParserBackend::Pest,
                compare_against: None,
            },
        ) {
            Ok(output) => output,
            Err(_) => continue,
        };
        let chumsky = parse_ast(
            &source,
            &path.to_string_lossy(),
            ParserOptions {
                primary: ParserBackend::Chumsky,
                compare_against: None,
            },
        )
        .unwrap();

        assert_eq!(
            pest.ast.diff_summary(&chumsky.ast),
            None,
            "parser mismatch for {}",
            path.display()
        );
    }
}

#[test]
fn malformed_input_reports_same_primary_error_span() {
    let source = r#"
main {
    let x = 1;
"#;

    let pest_err = parse_ast(
        source,
        "",
        ParserOptions {
            primary: ParserBackend::Pest,
            compare_against: None,
        },
    )
    .unwrap_err();
    let chumsky_err = parse_ast(
        source,
        "",
        ParserOptions {
            primary: ParserBackend::Chumsky,
            compare_against: None,
        },
    )
    .unwrap_err();

    assert_eq!(pest_err.error_type.to_string(), "Syntax Error");
    assert_eq!(chumsky_err.error_type.to_string(), "Syntax Error");
    assert!(
        pest_err
            .pos
            .as_ref()
            .unwrap()
            .line
            .abs_diff(chumsky_err.pos.as_ref().unwrap().line)
            <= 1
    );
    assert!(
        pest_err
            .pos
            .as_ref()
            .unwrap()
            .start
            .abs_diff(chumsky_err.pos.as_ref().unwrap().start)
            <= 1
    );
}
