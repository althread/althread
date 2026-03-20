use std::fs;

use althread::parser::parse_ast;

#[test]
fn chumsky_parses_basic_program() {
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

    parse_ast(source, "").unwrap();
}

#[test]
fn chumsky_parses_nested_header_types() {
    let source = r#"
program Worker(items: list(tuple(int, list(proc(Node))))) {
    let ready = true;
}

fn project(items: list(tuple(int, list(proc(Node))))) -> list(proc(Node)) {
    return items.at(0).at(1);
}

main {}
"#;

    parse_ast(source, "").unwrap();
}

#[test]
fn chumsky_parses_nested_channel_types() {
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

    parse_ast(source, "").unwrap();
}

#[test]
fn chumsky_parses_example_corpus() {
    for entry in fs::read_dir("../examples").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("alt") {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("TP2-communication.alt") {
            continue;
        }

        let source = fs::read_to_string(&path).unwrap();
        if source.trim().is_empty() {
            continue;
        }

        parse_ast(&source, &path.to_string_lossy()).unwrap();
    }
}

#[test]
fn malformed_input_reports_syntax_error() {
    let source = r#"
main {
    let x = 1;
"#;

    let err = parse_ast(source, "").unwrap_err();

    assert_eq!(err.error_type.to_string(), "Syntax Error");
    assert!(err.pos.is_some());
}
