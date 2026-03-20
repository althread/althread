use pest::{
    error::{ErrorVariant, InputLocation, LineColLocation},
    iterators::Pairs,
    Parser,
};
use pest_derive::Parser;

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
    error::{AlthreadError, AlthreadResult, ErrorType, Pos},
};

#[path = "parser/chumsky.rs"]
mod chumsky_backend;
#[path = "parser/syntax.rs"]
pub mod syntax;

use syntax::{SyntaxBlock, SyntaxBlockKind, SyntaxProgram};

#[derive(Parser)]
#[grammar = "althread.pest"]
struct AlthreadParser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserBackend {
    Pest,
    Chumsky,
}

#[derive(Debug, Clone, Copy)]
pub struct ParserOptions {
    pub primary: ParserBackend,
    pub compare_against: Option<ParserBackend>,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            primary: ParserBackend::Pest,
            compare_against: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseComparison {
    pub matched: bool,
    pub summary: Option<String>,
}

#[derive(Debug)]
pub struct ParseOutput {
    pub ast: Ast,
    pub comparison: Option<ParseComparison>,
}

pub fn parse<'a>(source: &'a str, file_path: &str) -> Result<Pairs<'a, Rule>, AlthreadError> {
    parse_rule(source, Rule::program, file_path)
}

pub(crate) fn parse_rule<'a>(
    source: &'a str,
    rule: Rule,
    file_path: &str,
) -> Result<Pairs<'a, Rule>, AlthreadError> {
    AlthreadParser::parse(rule, source).map_err(|e| map_pest_error(e, file_path))
}

pub fn parse_ast(
    source: &str,
    file_path: &str,
    options: ParserOptions,
) -> AlthreadResult<ParseOutput> {
    let primary_ast = parse_ast_with_backend(source, file_path, options.primary)?;
    let comparison = if let Some(other_backend) = options.compare_against {
        match parse_ast_with_backend(source, file_path, other_backend) {
            Ok(other_ast) => {
                let summary = primary_ast.diff_summary(&other_ast);
                Some(ParseComparison {
                    matched: summary.is_none(),
                    summary,
                })
            }
            Err(err) => Some(ParseComparison {
                matched: false,
                summary: Some(format!(
                    "comparison parser ({}) failed: {}",
                    other_backend.as_str(),
                    err.message
                )),
            }),
        }
    } else {
        None
    };

    Ok(ParseOutput {
        ast: primary_ast,
        comparison,
    })
}

pub(crate) fn parse_syntax_with_backend(
    source: &str,
    file_path: &str,
    backend: ParserBackend,
) -> AlthreadResult<SyntaxProgram> {
    match backend {
        ParserBackend::Pest => parse_syntax_with_pest(source, file_path),
        ParserBackend::Chumsky => chumsky_backend::parse_program(source, file_path),
    }
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
) -> AlthreadResult<(Pos, Vec<syntax::SyntaxSnippet>)> {
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

pub(crate) fn parse_ast_with_backend(
    source: &str,
    file_path: &str,
    backend: ParserBackend,
) -> AlthreadResult<Ast> {
    let syntax = parse_syntax_with_backend(source, file_path, backend)?;
    Ast::from_syntax(source, syntax, file_path)
}

fn parse_syntax_with_pest(source: &str, file_path: &str) -> AlthreadResult<SyntaxProgram> {
    let mut blocks = Vec::new();
    for pair in parse(source, file_path)? {
        match pair.as_rule() {
            Rule::import_block => blocks.push(SyntaxBlock::new(
                SyntaxBlockKind::Import,
                Pos::from_span(pair.as_span(), file_path),
                pair.as_str().to_string(),
            )),
            Rule::main_block => blocks.push(SyntaxBlock::new(
                SyntaxBlockKind::Main,
                Pos::from_span(pair.as_span(), file_path),
                pair.as_str().to_string(),
            )),
            Rule::global_block => blocks.push(SyntaxBlock::new(
                SyntaxBlockKind::Global,
                Pos::from_span(pair.as_span(), file_path),
                pair.as_str().to_string(),
            )),
            Rule::condition_block => {
                let kind = if pair.as_str().trim_start().starts_with("always") {
                    SyntaxBlockKind::Always
                } else {
                    SyntaxBlockKind::Never
                };
                blocks.push(SyntaxBlock::new(
                    kind,
                    Pos::from_span(pair.as_span(), file_path),
                    pair.as_str().to_string(),
                ));
            }
            Rule::check_block => blocks.push(SyntaxBlock::new(
                SyntaxBlockKind::Check,
                Pos::from_span(pair.as_span(), file_path),
                pair.as_str().to_string(),
            )),
            Rule::program_block => blocks.push(SyntaxBlock::new(
                SyntaxBlockKind::Program,
                Pos::from_span(pair.as_span(), file_path),
                pair.as_str().to_string(),
            )),
            Rule::function_block => blocks.push(SyntaxBlock::new(
                SyntaxBlockKind::Function,
                Pos::from_span(pair.as_span(), file_path),
                pair.as_str().to_string(),
            )),
            Rule::EOI => {}
            _ => {
                return Err(AlthreadError::new(
                    ErrorType::SyntaxError,
                    Some(Pos::from_span(pair.as_span(), file_path)),
                    format!(
                        "Unexpected rule {:?} while collecting syntax blocks",
                        pair.as_rule()
                    ),
                ));
            }
        }
    }

    Ok(SyntaxProgram { blocks })
}

fn map_pest_error(e: pest::error::Error<Rule>, file_path: &str) -> AlthreadError {
    let mut pos = match e.line_col {
        LineColLocation::Pos(pos) | LineColLocation::Span(pos, _) => Pos {
            line: pos.0,
            col: pos.1,
            start: 0,
            end: 0,
            file_path: file_path.to_string(),
        },
    };
    match e.location {
        InputLocation::Pos(p) => {
            pos.start = p;
            pos.end = p + 1;
        }
        InputLocation::Span((start, end)) => {
            pos.start = start;
            pos.end = end;
        }
    };

    let error_message = match e.variant {
        ErrorVariant::ParsingError { positives, .. } => {
            format!("Expected one of {:?}", positives)
        }
        ErrorVariant::CustomError { message } => message,
    };
    AlthreadError::new(ErrorType::SyntaxError, Some(pos), error_message)
}

impl ParserBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pest => "pest",
            Self::Chumsky => "chumsky",
        }
    }
}
