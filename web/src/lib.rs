use wasm_bindgen::prelude::*;


use althread::{ast::Ast, error::AlthreadError};

fn error_to_js(err: AlthreadError) -> JsValue {
    serde_wasm_bindgen::to_value(&err).unwrap()
}


#[wasm_bindgen]
pub fn compile(source: &str) -> Result<String,JsValue> {
    
    // parse code with pest
    let pairs = althread::parser::parse(&source).map_err(error_to_js)?;

    let ast = Ast::build(pairs).map_err(error_to_js)?;

    println!("{}", &ast);

    let compiled_project = ast.compile().map_err(error_to_js)?;

    Ok(format!("{}", compiled_project))
}
