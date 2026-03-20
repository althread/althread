use crate::{
    ast::{
        import_block::ImportBlock,
        node::Node,
        statement::{
            channel_declaration::ChannelDeclaration,
            expression::{Expression, SideEffectExpression},
            fn_call::FnCall,
            run_call::RunCall,
            send::SendStatement,
        },
        token::{args_list::ArgsList, datatype::DataType, object_identifier::ObjectIdentifier},
        Ast,
    },
    error::AlthreadResult,
};

#[path = "parser/chumsky.rs"]
mod chumsky_backend;
#[path = "parser/syntax.rs"]
pub mod syntax;

pub fn parse_ast(source: &str, file_path: &str) -> AlthreadResult<Ast> {
    let syntax = chumsky_backend::parse_program(source, file_path)?;
    Ast::from_syntax(source, syntax, file_path)
}

pub(crate) fn parse_args_list_with_chumsky(
    source: &str,
    snippet: &syntax::SyntaxSnippet,
    file_path: &str,
) -> AlthreadResult<Node<ArgsList>> {
    chumsky_backend::parse_args_list(source, snippet, file_path)
}

pub(crate) fn parse_datatype_with_chumsky(
    source: &str,
    snippet: &syntax::SyntaxSnippet,
    file_path: &str,
) -> AlthreadResult<Node<DataType>> {
    chumsky_backend::parse_datatype(source, snippet, file_path)
}

pub(crate) fn parse_statement_block_with_chumsky(
    source: &str,
    snippet: &syntax::SyntaxSnippet,
    file_path: &str,
) -> AlthreadResult<(crate::error::Pos, Vec<syntax::SyntaxSnippet>)> {
    chumsky_backend::parse_statement_block(source, snippet, file_path)
}

pub(crate) fn parse_object_identifier_with_chumsky(
    source: &str,
    snippet: &syntax::SyntaxSnippet,
    file_path: &str,
) -> AlthreadResult<Node<ObjectIdentifier>> {
    chumsky_backend::parse_object_identifier(source, snippet, file_path)
}

pub(crate) fn parse_fn_call_with_chumsky(
    source: &str,
    snippet: &syntax::SyntaxSnippet,
    file_path: &str,
) -> AlthreadResult<Node<FnCall>> {
    chumsky_backend::parse_fn_call(source, snippet, file_path)
}

pub(crate) fn parse_run_call_with_chumsky(
    source: &str,
    snippet: &syntax::SyntaxSnippet,
    file_path: &str,
) -> AlthreadResult<Node<RunCall>> {
    chumsky_backend::parse_run_call(source, snippet, file_path)
}

pub(crate) fn parse_send_call_with_chumsky(
    source: &str,
    snippet: &syntax::SyntaxSnippet,
    file_path: &str,
) -> AlthreadResult<Node<SendStatement>> {
    chumsky_backend::parse_send_call(source, snippet, file_path)
}

pub(crate) fn parse_channel_declaration_with_chumsky(
    source: &str,
    snippet: &syntax::SyntaxSnippet,
    file_path: &str,
) -> AlthreadResult<Node<ChannelDeclaration>> {
    chumsky_backend::parse_channel_declaration(source, snippet, file_path)
}

pub(crate) fn parse_import_block_with_chumsky(
    source: &str,
    snippet: &syntax::SyntaxSnippet,
    file_path: &str,
) -> AlthreadResult<Node<ImportBlock>> {
    chumsky_backend::parse_import_block(source, snippet, file_path)
}

pub(crate) fn parse_expression_with_chumsky(
    source: &str,
    snippet: &syntax::SyntaxSnippet,
    file_path: &str,
) -> AlthreadResult<Node<Expression>> {
    chumsky_backend::parse_expression(source, snippet, file_path)
}

pub(crate) fn parse_side_effect_expression_with_chumsky(
    source: &str,
    snippet: &syntax::SyntaxSnippet,
    file_path: &str,
) -> AlthreadResult<Node<SideEffectExpression>> {
    chumsky_backend::parse_side_effect_expression(source, snippet, file_path)
}

pub(crate) fn parse_list_expression_with_chumsky(
    source: &str,
    snippet: &syntax::SyntaxSnippet,
    file_path: &str,
) -> AlthreadResult<Node<Expression>> {
    chumsky_backend::parse_list_expression(source, snippet, file_path)
}
