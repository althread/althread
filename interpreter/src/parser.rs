use crate::{ast::Ast, error::AlthreadResult};

#[path = "parser/chumsky_combinator.rs"]
pub mod chumsky_backend;
#[path = "parser/syntax.rs"]
pub mod syntax;
pub use chumsky_backend as chumsky_combinator;

pub fn parse_ast(source: &str, file_path: &str) -> AlthreadResult<Ast> {
    chumsky_backend::parse_ast(source, file_path)
}
