use crate::{ast::Ast, error::AlthreadResult};

#[path = "parser/chumsky_combinator.rs"]
pub mod chumsky_backend;
#[path = "parser/syntax.rs"]
pub mod syntax;
pub use chumsky_backend as chumsky_combinator;
pub(crate) use chumsky_backend::{
    parse_expression as parse_expression_with_chumsky,
    parse_list_expression as parse_list_expression_with_chumsky,
};

pub fn parse_ast(source: &str, file_path: &str) -> AlthreadResult<Ast> {
    chumsky_backend::parse_ast(source, file_path)
}
