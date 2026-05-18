use crate::{
    ast::Ast,
    error::{register_source, AlthreadResult},
};

#[path = "parser/chumsky_combinator.rs"]
pub mod chumsky_backend;
#[path = "parser/lexer.rs"]
pub mod lexer;
#[path = "parser/syntax.rs"]
pub mod syntax;
pub use chumsky_backend as chumsky_combinator;

pub fn parse_ast(source: &str, file_path: &str) -> AlthreadResult<Ast> {
    register_source(file_path, source);
    chumsky_backend::parse_ast(source, file_path)
}
