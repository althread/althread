use wasm_bindgen::prelude::*;


use althread::ast::Ast;


#[wasm_bindgen]
pub fn compile(source: &str) -> Result<String,String> {
    
    // parse code with pest
    let pairs = althread::parser::parse(&source).map_err(|e| {
        format!("{:?}", e)
    })?;

    let ast = Ast::build(pairs).map_err(|e| {
        format!("{:?}", e)
    })?;

    println!("{}", &ast);

    let compiled_project = ast.compile().map_err(|e| {
        format!("{:?}", e)
    })?;

    Ok(format!("{}", compiled_project))
}
