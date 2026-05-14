use chumsky::{extra, input::ValueInput, pratt::*, prelude::*, primitive::todo};
use logos::Logos;
use ordered_float::OrderedFloat;

use crate::{
    ast::{
        import_block::ImportBlock,
        node::Node,
        statement::{
            channel_declaration::ChannelDeclaration,
            expression::{
                primary_expression::PrimaryExpression, tuple_expression::TupleExpression,
                BracketContent, BracketExpression, Expression, SideEffectExpression,
            },
            fn_call::FnCall,
            run_call::RunCall,
            send::SendStatement,
        },
        token::{
            args_list::ArgsList, datatype::DataType, identifier::Identifier, literal::Literal,
            object_identifier::ObjectIdentifier,
        },
    },
    error::{AlthreadError, ErrorType, Pos},
    parser::syntax::{
        SyntaxBlock, SyntaxBlockDetail, SyntaxBlockKind, SyntaxProgram, SyntaxSnippet,
    },
};

pub type Span = SimpleSpan<usize>;
type Spanned<T> = (T, Span);
type ParserExtra<'a> = extra::Err<Rich<'a, Token, Span>>;

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip r"//[^\n]*")]
pub enum Token {
    #[token("@private")]
    AtPrivate,
    #[token("shared")]
    Shared,
    #[token("main")]
    Main,
    #[token("always")]
    Always,
    #[token("never")]
    Never,
    #[token("check")]
    Check,
    #[token("program")]
    Program,
    #[token("fn")]
    Fn,
    #[token("let")]
    Let,
    #[token("const")]
    Const,
    #[token("proc")]
    Proc,
    #[token("list")]
    List,
    #[token("tuple")]
    Tuple,
    #[token("bool")]
    BoolType,
    #[token("int")]
    IntType,
    #[token("float")]
    FloatType,
    #[token("string")]
    StringType,
    #[token("void")]
    VoidType,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("null")]
    Null,
    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice().to_string())]
    StringLiteral(String),
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().to_string())]
    FloatLiteral(String),
    #[regex(r"[0-9]+", |lex| lex.slice().to_string())]
    IntLiteral(String),
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),
    #[token("->")]
    Arrow,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("=")]
    Eq,
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semi,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
enum PrototypeExpr {
    Name(String),
    Int(String),
    Bool(bool),
    Str(String),
    Null,
    Add(Box<Self>, Box<Self>),
    Sub(Box<Self>, Box<Self>),
    Mul(Box<Self>, Box<Self>),
    Div(Box<Self>, Box<Self>),
    Neg(Box<Self>),
}

pub fn parse_program(source: &str, file_path: &str) -> Result<SyntaxProgram, AlthreadError> {
    let tokens = lex(source, file_path)?;
    let eoi = Span::new((), source.len()..source.len());
    let parser = program_parser(source, file_path);
    let input = tokens.as_slice().map(eoi, |(token, span)| (token, span));

    parser
        .parse(input)
        .into_result()
        .map_err(|errs| map_rich_errors(source, file_path, errs))
}

pub fn parse_args_list(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<ArgsList>, AlthreadError> {
    Err(not_implemented_error(
        source,
        file_path,
        snippet.pos.clone(),
        "args list parsing is not implemented in chumsky_combinator yet",
    ))
}

pub fn parse_statement_block(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<(Pos, Vec<SyntaxSnippet>), AlthreadError> {
    Err(not_implemented_error(
        source,
        file_path,
        snippet.pos.clone(),
        "statement block parsing is not implemented in chumsky_combinator yet",
    ))
}

pub fn parse_import_block(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<ImportBlock>, AlthreadError> {
    Err(not_implemented_error(
        source,
        file_path,
        snippet.pos.clone(),
        "import block parsing is not implemented in chumsky_combinator yet",
    ))
}

pub fn parse_fn_call(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<FnCall>, AlthreadError> {
    Err(not_implemented_error(
        source,
        file_path,
        snippet.pos.clone(),
        "function call parsing is not implemented in chumsky_combinator yet",
    ))
}

pub fn parse_run_call(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<RunCall>, AlthreadError> {
    Err(not_implemented_error(
        source,
        file_path,
        snippet.pos.clone(),
        "run call parsing is not implemented in chumsky_combinator yet",
    ))
}

pub fn parse_send_call(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<SendStatement>, AlthreadError> {
    Err(not_implemented_error(
        source,
        file_path,
        snippet.pos.clone(),
        "send call parsing is not implemented in chumsky_combinator yet",
    ))
}

pub fn parse_channel_declaration(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<ChannelDeclaration>, AlthreadError> {
    Err(not_implemented_error(
        source,
        file_path,
        snippet.pos.clone(),
        "channel declaration parsing is not implemented in chumsky_combinator yet",
    ))
}

pub fn parse_datatype(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<DataType>, AlthreadError> {
    let tokens = lex_snippet(snippet, file_path)?;
    let eoi = Span::new((), snippet.pos.end..snippet.pos.end);
    let parser = datatype_parser();
    let input = tokens.as_slice().map(eoi, |(token, span)| (token, span));
    parser
        .parse(input)
        .into_result()
        .map_err(|errs| map_rich_errors(source, file_path, errs))
}

pub fn parse_object_identifier(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<ObjectIdentifier>, AlthreadError> {
    let tokens = lex_snippet(snippet, file_path)?;
    let eoi = Span::new((), snippet.pos.end..snippet.pos.end);
    let parser = object_identifier_parser();
    let input = tokens.as_slice().map(eoi, |(token, span)| (token, span));
    parser
        .parse(input)
        .into_result()
        .map_err(|errs| map_rich_errors(source, file_path, errs))
}

pub fn parse_expression(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<Expression>, AlthreadError> {
    let tokens = lex_snippet(snippet, file_path)?;
    let eoi = Span::new((), snippet.pos.end..snippet.pos.end);
    let parser = expression_parser();
    let input = tokens.as_slice().map(eoi, |(token, span)| (token, span));
    parser
        .parse(input)
        .into_result()
        .map_err(|errs| map_rich_errors(source, file_path, errs))
}

pub fn parse_side_effect_expression(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<SideEffectExpression>, AlthreadError> {
    let tokens = lex_snippet(snippet, file_path)?;
    let eoi = Span::new((), snippet.pos.end..snippet.pos.end);
    let parser = side_effect_expression_parser();
    let input = tokens.as_slice().map(eoi, |(token, span)| (token, span));
    parser
        .parse(input)
        .into_result()
        .map_err(|errs| map_rich_errors(source, file_path, errs))
}

pub fn parse_list_expression(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<Expression>, AlthreadError> {
    parse_expression(source, snippet, file_path)
}

fn lex(source: &str, file_path: &str) -> Result<Vec<Spanned<Token>>, AlthreadError> {
    let mut lexer = Token::lexer(source);
    let mut tokens = Vec::new();
    while let Some(result) = lexer.next() {
        let span = lexer.span();
        match result {
            Ok(token) => tokens.push((token, Span::new((), span.start..span.end))),
            Err(()) => {
                return Err(AlthreadError::new(
                    ErrorType::SyntaxError,
                    Some(Pos::from_offsets(source, file_path, span.start, span.end)),
                    format!("invalid token '{}'", &source[span.start..span.end]),
                ));
            }
        }
    }
    Ok(tokens)
}

fn program_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, SyntaxProgram, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    choice((
        shared_block_parser(source, file_path).boxed(),
        unsupported_block_parser(Token::Main).boxed(),
        unsupported_block_parser(Token::Always).boxed(),
        unsupported_block_parser(Token::Never).boxed(),
        unsupported_block_parser(Token::Check).boxed(),
        unsupported_prefixed_block_parser(Token::Program).boxed(),
        unsupported_prefixed_block_parser(Token::Fn).boxed(),
    ))
    .repeated()
    .collect::<Vec<_>>()
    .then_ignore(end())
    .map(|blocks| SyntaxProgram { blocks })
}

fn shared_block_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, SyntaxBlock, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    just(Token::Shared)
        .ignore_then(
            shared_declaration_parser(source, file_path)
                .repeated()
                .collect::<Vec<_>>()
                .map_with(move |body, e| (pos_from_span(source, file_path, e.span()), body))
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        )
        .map_with(move |(body_pos, body), e| SyntaxBlock {
            kind: SyntaxBlockKind::Global,
            pos: pos_from_span(source, file_path, e.span()),
            text: slice_from_span(source, e.span()).to_string(),
            detail: SyntaxBlockDetail::Global { body_pos, body },
        })
}

fn shared_declaration_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, SyntaxSnippet, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    let ident = select! { Token::Ident(name) => name }.labelled("identifier");

    let object_ident = ident
        .clone()
        .separated_by(just(Token::Dot))
        .at_least(1)
        .collect::<Vec<_>>();

    let datatype = recursive(|datatype| {
        let primitive = choice((
            just(Token::BoolType),
            just(Token::IntType),
            just(Token::FloatType),
            just(Token::StringType),
            just(Token::VoidType),
        ))
        .ignored();

        let proc = just(Token::Proc)
            .ignore_then(
                object_ident
                    .clone()
                    .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .ignored();

        let list = just(Token::List)
            .ignore_then(
                datatype
                    .clone()
                    .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .ignored();

        let tuple = just(Token::Tuple)
            .ignore_then(
                datatype
                    .clone()
                    .separated_by(just(Token::Comma))
                    .at_least(1)
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .ignored();

        choice((primitive, proc, list, tuple)).labelled("datatype")
    });

    let initializer = raw_initializer_parser();

    choice((just(Token::Let), just(Token::Const)))
        .ignored()
        .then(ident)
        .then(just(Token::Colon).ignore_then(datatype.clone()).or_not())
        .then(just(Token::Eq).ignore_then(initializer).or_not())
        .then_ignore(just(Token::Semi))
        .to_span()
        .map(move |span| {
            SyntaxSnippet::new(
                pos_from_span(source, file_path, span),
                slice_from_span(source, span).to_string(),
            )
        })
}

fn datatype_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<DataType>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    recursive(|datatype| {
        let primitive = choice((
            just(Token::BoolType).to(DataType::Boolean),
            just(Token::IntType).to(DataType::Integer),
            just(Token::FloatType).to(DataType::Float),
            just(Token::StringType).to(DataType::String),
            just(Token::VoidType).to(DataType::Void),
        ))
        .map_with(|value, e| Node {
            pos: pos_from_span_source(e.span()),
            value,
        });

        let proc = just(Token::Proc)
            .ignore_then(
                object_identifier_parser().delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .map_with(|name, e| Node {
                pos: pos_from_span_source(e.span()),
                value: DataType::Process(name.value.to_string()),
            });

        let list = just(Token::List)
            .ignore_then(
                datatype
                    .clone()
                    .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .map_with(|inner: Node<DataType>, e| Node {
                pos: pos_from_span_source(e.span()),
                value: DataType::List(Box::new(inner.value)),
            });

        let tuple = just(Token::Tuple)
            .ignore_then(
                datatype
                    .clone()
                    .separated_by(just(Token::Comma))
                    .at_least(1)
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .map_with(|items, e| Node {
                pos: pos_from_span_source(e.span()),
                value: DataType::Tuple(items.into_iter().map(|item| item.value).collect()),
            });

        choice((primitive, proc, list, tuple))
    })
}

fn object_identifier_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<ObjectIdentifier>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    select! { Token::Ident(name) => name }
        .map_with(|name, e| Node {
            pos: pos_from_span_source(e.span()),
            value: Identifier { value: name },
        })
        .separated_by(just(Token::Dot))
        .at_least(1)
        .collect::<Vec<_>>()
        .map_with(|parts, e| Node {
            pos: pos_from_span_source(e.span()),
            value: ObjectIdentifier { parts },
        })
}

fn literal_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<Literal>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    select! {
        Token::True => Literal::Bool(true),
        Token::False => Literal::Bool(false),
        Token::Null => Literal::Null,
        Token::IntLiteral(value) => Literal::Int(value.parse::<i64>().expect("valid int literal")),
        Token::FloatLiteral(value) => Literal::Float(OrderedFloat(value.parse::<f64>().expect("valid float literal"))),
        Token::StringLiteral(value) => Literal::String(unquote_string(&value)),
    }
    .map_with(|value, e| Node {
        pos: pos_from_span_source(e.span()),
        value,
    })
}

fn expression_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<Expression>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    recursive(|expr| {
        let literal = literal_parser().map(|literal| Node {
            pos: literal.pos.clone(),
            value: Expression::Primary(Node {
                pos: literal.pos.clone(),
                value: PrimaryExpression::Literal(literal),
            }),
        });

        let ident = object_identifier_parser().map(|ident| Node {
            pos: ident.pos.clone(),
            value: Expression::Primary(Node {
                pos: ident.pos.clone(),
                value: PrimaryExpression::Identifier(ident),
            }),
        });

        let tuple = expr
            .clone()
            .separated_by(just(Token::Comma))
            .at_least(1)
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .map_with(|values, e| Node {
                pos: pos_from_span_source(e.span()),
                value: Expression::Tuple(Node {
                    pos: pos_from_span_source(e.span()),
                    value: TupleExpression { values },
                }),
            });

        let grouped = expr
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .map_with(|inner, e| Node {
                pos: pos_from_span_source(e.span()),
                value: Expression::Primary(Node {
                    pos: pos_from_span_source(e.span()),
                    value: PrimaryExpression::Expression(Box::new(inner)),
                }),
            });

        choice((literal, tuple, grouped, ident)).boxed()
    })
}

fn side_effect_expression_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<SideEffectExpression>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    let list_items = recursive(|side_effect| {
        side_effect
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LBracket), just(Token::RBracket))
            .map_with(|items, e| Node {
                pos: pos_from_span_source(e.span()),
                value: SideEffectExpression::Bracket(Node {
                    pos: pos_from_span_source(e.span()),
                    value: BracketExpression {
                        content: BracketContent::ListLiteral(items),
                    },
                }),
            })
            .or(expression_parser().map(|expr| Node {
                pos: expr.pos.clone(),
                value: SideEffectExpression::Expression(expr),
            }))
    });

    list_items
}

fn raw_initializer_parser<'tokens, I>() -> impl Parser<'tokens, I, (), ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    recursive(|value| {
        let atom = select! {
            Token::Ident(_) => (),
            Token::IntLiteral(_) => (),
            Token::FloatLiteral(_) => (),
            Token::StringLiteral(_) => (),
            Token::True => (),
            Token::False => (),
            Token::Null => (),
            Token::Plus => (),
            Token::Minus => (),
            Token::Star => (),
            Token::Slash => (),
            Token::EqEq => (),
            Token::NotEq => (),
            Token::AndAnd => (),
            Token::OrOr => (),
            Token::Dot => (),
            Token::Comma => (),
            Token::Colon => (),
            Token::Lt => (),
            Token::Gt => (),
        }
        .ignored();

        choice((
            atom,
            value
                .clone()
                .repeated()
                .delimited_by(just(Token::LParen), just(Token::RParen))
                .ignored(),
            value
                .clone()
                .repeated()
                .delimited_by(just(Token::LBracket), just(Token::RBracket))
                .ignored(),
            value
                .clone()
                .repeated()
                .delimited_by(just(Token::LBrace), just(Token::RBrace))
                .ignored(),
        ))
    })
    .repeated()
    .at_least(1)
    .ignored()
}

#[allow(dead_code)]
fn prototype_expression_parser<'tokens, I>(
) -> impl Parser<'tokens, I, PrototypeExpr, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    let atom = select! {
        Token::Ident(name) => PrototypeExpr::Name(name),
        Token::IntLiteral(value) => PrototypeExpr::Int(value),
        Token::StringLiteral(value) => PrototypeExpr::Str(value),
        Token::True => PrototypeExpr::Bool(true),
        Token::False => PrototypeExpr::Bool(false),
        Token::Null => PrototypeExpr::Null,
    };

    atom.pratt((
        prefix(3, just(Token::Minus), |_, rhs, _| {
            PrototypeExpr::Neg(Box::new(rhs))
        }),
        infix(left(2), just(Token::Star), |l, _, r, _| {
            PrototypeExpr::Mul(Box::new(l), Box::new(r))
        }),
        infix(left(2), just(Token::Slash), |l, _, r, _| {
            PrototypeExpr::Div(Box::new(l), Box::new(r))
        }),
        infix(left(1), just(Token::Plus), |l, _, r, _| {
            PrototypeExpr::Add(Box::new(l), Box::new(r))
        }),
        infix(left(1), just(Token::Minus), |l, _, r, _| {
            PrototypeExpr::Sub(Box::new(l), Box::new(r))
        }),
    ))
}

fn unsupported_block_parser<'tokens, I>(
    token: Token,
) -> impl Parser<'tokens, I, SyntaxBlock, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    just(token).ignore_then(todo())
}

fn unsupported_prefixed_block_parser<'tokens, I>(
    token: Token,
) -> impl Parser<'tokens, I, SyntaxBlock, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    just(Token::AtPrivate)
        .or_not()
        .ignore_then(just(token))
        .ignore_then(todo())
}

fn lex_snippet(
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Vec<Spanned<Token>>, AlthreadError> {
    let mut lexer = Token::lexer(snippet.text.as_str());
    let mut tokens = Vec::new();
    while let Some(result) = lexer.next() {
        let span = lexer.span();
        match result {
            Ok(token) => tokens.push((
                token,
                Span::new(
                    (),
                    (snippet.pos.start + span.start)..(snippet.pos.start + span.end),
                ),
            )),
            Err(()) => {
                return Err(AlthreadError::new(
                    ErrorType::SyntaxError,
                    Some(Pos::from_offsets(
                        &snippet.text,
                        file_path,
                        span.start,
                        span.end,
                    )),
                    format!("invalid token '{}'", &snippet.text[span.start..span.end]),
                ));
            }
        }
    }
    Ok(tokens)
}

fn unquote_string(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(value)
        .to_string()
}

fn not_implemented_error(
    _source: &str,
    _file_path: &str,
    pos: Pos,
    message: &str,
) -> AlthreadError {
    AlthreadError::new(ErrorType::SyntaxError, Some(pos), message.to_string())
}

fn pos_from_span_source(span: Span) -> Pos {
    Pos {
        line: 0,
        col: 0,
        start: span.start,
        end: span.end,
        file_path: String::new(),
    }
}

fn pos_from_span(source: &str, file_path: &str, span: Span) -> Pos {
    Pos::from_offsets(source, file_path, span.start, span.end)
}

fn slice_from_span(source: &str, span: Span) -> &str {
    &source[span.start..span.end]
}

fn map_rich_errors(
    source: &str,
    file_path: &str,
    errs: Vec<Rich<'_, Token, Span>>,
) -> AlthreadError {
    let err = errs.into_iter().next().expect("chumsky returned no errors");
    let message = format!("{:?}", err.reason());
    AlthreadError::new(
        ErrorType::SyntaxError,
        Some(Pos::from_offsets(
            source,
            file_path,
            err.span().start,
            err.span().end,
        )),
        message,
    )
}
