use fastrand;
use wasm_bindgen::prelude::*;

use althread::{ast::Ast, error::AlthreadError};

fn error_to_js(err: AlthreadError) -> JsValue {
    serde_wasm_bindgen::to_value(&err).unwrap()
}

#[wasm_bindgen]
pub fn compile(source: &str) -> Result<String, JsValue> {
    // parse code with pest
    let pairs = althread::parser::parse(&source).map_err(error_to_js)?;

    let ast = Ast::build(pairs).map_err(error_to_js)?;

    println!("{}", &ast);

    let compiled_project = ast.compile().map_err(error_to_js)?;

    Ok(format!("{}", compiled_project))
}

#[wasm_bindgen]
pub fn run(source: &str) -> Result<String, JsValue> {
    // parse code with pest
    let pairs = althread::parser::parse(&source).map_err(error_to_js)?;

    let ast = Ast::build(pairs).map_err(error_to_js)?;

    println!("{}", &ast);

    let compiled_project = ast.compile().map_err(error_to_js)?;

    let mut vm = althread::vm::VM::new(&compiled_project);

    vm.start(fastrand::u64(0..(1 << 32)));

    let mut result = String::new();

    for _ in 0..100000 {
        if vm.is_finished() {
            break;
        }
        let info = vm.next_random().map_err(error_to_js)?;
        for inst in info.instructions.iter() {
            result.push_str(&format!("#{}: {}\n", info.prog_id, inst));
        }
        if info.invariant_error.is_err() {
            let err = info.invariant_error.unwrap_err();
            result.push_str(&format!("Invariant error at line {}: {}\n", err.pos.unwrap().line, err.message));
            break;
        }
    }
    Ok(result)
}
