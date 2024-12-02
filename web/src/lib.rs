use fastrand;
use wasm_bindgen::prelude::*;
use serde::ser::{Serialize, Serializer, SerializeStruct};

use althread::{ast::Ast, checker, error::AlthreadError, vm::GlobalAction};

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

pub struct RunResult {
    debug: String,
    stdout: Vec<String>,
}

impl Serialize for RunResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("RunResult", 2)?;
        state.serialize_field("debug", &self.debug)?;
        state.serialize_field("stdout", &self.stdout)?;
        state.end()
    }
}

#[wasm_bindgen]
pub fn run(source: &str) -> Result<JsValue, JsValue> {
    // parse code with pest
    let pairs = althread::parser::parse(&source).map_err(error_to_js)?;

    let ast = Ast::build(pairs).map_err(error_to_js)?;

    println!("{}", &ast);

    let compiled_project = ast.compile().map_err(error_to_js)?;

    let mut vm = althread::vm::VM::new(&compiled_project);

    vm.start(fastrand::u64(0..(1 << 32)));

    let mut result = String::new();
    let mut stdout = vec![];

    for _ in 0..100000 {
        if vm.is_finished() {
            break;
        }
        let info = vm.next_random().map_err(error_to_js)?;
        for inst in info.instructions.iter() {
            result.push_str(&format!("#{}: {}\n", info.prog_id, inst));
        }
        for p in info.actions.iter() {
            if let GlobalAction::Print(s) = p {
                stdout.push(s.clone());
            }
        }
        if info.invariant_error.is_err() {
            let err = info.invariant_error.unwrap_err();
            result.push_str(&format!("Invariant error at line {}: {}\n", err.pos.unwrap().line, err.message));
            break;
        }
    }
    Ok(serde_wasm_bindgen::to_value(&RunResult {
        debug: result,
        stdout,
    }).unwrap())
}



#[wasm_bindgen]
pub fn check(source: &str) -> Result<JsValue, JsValue> {
    // parse code with pest
    let pairs = althread::parser::parse(&source).map_err(error_to_js)?;

    let ast = Ast::build(pairs).map_err(error_to_js)?;

    println!("{}", &ast);

    let compiled_project = ast.compile().map_err(error_to_js)?;


    let checked = checker::check_program(&compiled_project).map_err(error_to_js)?;

    Ok(serde_wasm_bindgen::to_value(&checked).unwrap())
    
}
