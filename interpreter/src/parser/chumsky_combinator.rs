use chumsky::{
    error::{RichPattern, RichReason},
    extra,
    input::ValueInput,
    pratt::*,
    prelude::*,
};
use logos::Logos;
use ordered_float::OrderedFloat;
use std::fmt;

use crate::{
    ast::{
        block::Block,
        node::Node,
        statement::{
            declaration::Declaration,
            expression::{
                binary_expression::BinaryExpression, primary_expression::PrimaryExpression,
                tuple_expression::TupleExpression, unary_expression::UnaryExpression,
                BracketContent, BracketExpression, CallChainExpression, CallChainSegment,
                Expression,
            },
            Statement,
        },
        token::{
            binary_operator::BinaryOperator, datatype::DataType,
            declaration_keyword::DeclarationKeyword, identifier::Identifier, literal::Literal,
            object_identifier::ObjectIdentifier, unary_operator::UnaryOperator,
        },
        Ast,
    },
    error::{AlthreadError, ErrorType, Pos},
    parser::syntax::SyntaxSnippet,
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
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("<<")]
    ShiftLeft,
    #[token(">>")]
    ShiftRight,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("!")]
    Bang,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("&")]
    Amp,
    #[token("|")]
    Pipe,
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

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::AtPrivate => write!(f, "@private"),
            Token::Shared => write!(f, "shared"),
            Token::Main => write!(f, "main"),
            Token::Always => write!(f, "always"),
            Token::Never => write!(f, "never"),
            Token::Check => write!(f, "check"),
            Token::Program => write!(f, "program"),
            Token::Fn => write!(f, "fn"),
            Token::Let => write!(f, "let"),
            Token::Const => write!(f, "const"),
            Token::Proc => write!(f, "proc"),
            Token::List => write!(f, "list"),
            Token::Tuple => write!(f, "tuple"),
            Token::BoolType => write!(f, "bool"),
            Token::IntType => write!(f, "int"),
            Token::FloatType => write!(f, "float"),
            Token::StringType => write!(f, "string"),
            Token::VoidType => write!(f, "void"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Null => write!(f, "null"),
            Token::StringLiteral(_) => write!(f, "string literal"),
            Token::FloatLiteral(_) => write!(f, "float literal"),
            Token::IntLiteral(_) => write!(f, "int literal"),
            Token::Ident(name) => write!(f, "{name}"),
            Token::Arrow => write!(f, "->"),
            Token::EqEq => write!(f, "=="),
            Token::NotEq => write!(f, "!="),
            Token::LtEq => write!(f, "<="),
            Token::GtEq => write!(f, ">="),
            Token::ShiftLeft => write!(f, "<<"),
            Token::ShiftRight => write!(f, ">>"),
            Token::AndAnd => write!(f, "&&"),
            Token::OrOr => write!(f, "||"),
            Token::Bang => write!(f, "!"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Amp => write!(f, "&"),
            Token::Pipe => write!(f, "|"),
            Token::Eq => write!(f, "="),
            Token::Dot => write!(f, "."),
            Token::Comma => write!(f, ","),
            Token::Colon => write!(f, ":"),
            Token::Semi => write!(f, ";"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
        }
    }
}

#[derive(Debug)]
enum TopLevelBlock {
    Shared(Node<Block>),
}

pub fn parse_program(source: &str, file_path: &str) -> Result<Ast, AlthreadError> {
    let tokens = lex(source, file_path)?;
    let eoi = Span::new((), source.len()..source.len());
    let parser = ast_parser(source, file_path);
    let input = tokens.as_slice().map(eoi, |(token, span)| (token, span));

    parser
        .parse(input)
        .into_result()
        .map_err(|errs| map_rich_errors(source, file_path, errs))
}

pub fn parse_ast(source: &str, file_path: &str) -> Result<Ast, AlthreadError> {
    parse_program(source, file_path)
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

fn ast_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, Ast, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    choice((
        shared_block_parser(source, file_path).boxed(),
        unsupported_block_parser(Token::Main, "main").boxed(),
        unsupported_block_parser(Token::Always, "always").boxed(),
        unsupported_block_parser(Token::Never, "never").boxed(),
        unsupported_block_parser(Token::Check, "check").boxed(),
        unsupported_prefixed_block_parser(Token::Program, "program").boxed(),
        unsupported_prefixed_block_parser(Token::Fn, "function").boxed(),
    ))
    .repeated()
    .collect::<Vec<TopLevelBlock>>()
    .then_ignore(end())
    .map(|blocks| {
        let mut ast = Ast::new();
        for block in blocks {
            match block {
                TopLevelBlock::Shared(block) => ast.global_block = Some(block),
            }
        }
        ast
    })
}

fn shared_block_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, TopLevelBlock, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    just(Token::Shared)
        .ignore_then(
            declaration_statement_parser()
                .repeated()
                .collect::<Vec<_>>()
                .map_with(move |children, e| Node {
                    pos: pos_from_span(source, file_path, e.span()),
                    value: Block { children },
                })
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        )
        .map(TopLevelBlock::Shared)
}

fn declaration_statement_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<Statement>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    let keyword = choice((
        just(Token::Let).to(DeclarationKeyword::Let),
        just(Token::Const).to(DeclarationKeyword::Const),
    ))
    .map_with(|value, e| Node {
        pos: pos_from_span_source(e.span()),
        value,
    });

    keyword
        .then(object_identifier_parser())
        .then(just(Token::Colon).ignore_then(datatype_parser()).or_not())
        .then(just(Token::Eq).ignore_then(expression_parser()).or_not())
        .then_ignore(just(Token::Semi))
        .map_with(|(((keyword, identifier), datatype), value), e| Node {
            pos: pos_from_span_source(e.span()),
            value: Statement::Declaration(Node {
                pos: pos_from_span_source(e.span()),
                value: Declaration {
                    keyword,
                    identifier,
                    datatype,
                    value,
                },
            }),
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
    identifier_parser()
        .separated_by(just(Token::Dot))
        .at_least(1)
        .collect::<Vec<_>>()
        .map_with(|parts, e| Node {
            pos: pos_from_span_source(e.span()),
            value: ObjectIdentifier { parts },
        })
}

fn identifier_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<Identifier>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    select! { Token::Ident(name) => name }.map_with(|name, e| Node {
        pos: pos_from_span_source(e.span()),
        value: Identifier { value: name },
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
        let args_tuple = expr
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .map_with(|values, e| tuple_expression_node(values, e.span()));

        let literal = literal_parser().map(|literal| Node {
            pos: literal.pos.clone(),
            value: Expression::Primary(Node {
                pos: literal.pos.clone(),
                value: PrimaryExpression::Literal(literal),
            }),
        });

        let ident = identifier_parser().map(|ident| {
            let pos = ident.pos.clone();
            let object_identifier = Node {
                pos: pos.clone(),
                value: ObjectIdentifier { parts: vec![ident] },
            };
            Node {
                pos: pos.clone(),
                value: Expression::Primary(Node {
                    pos,
                    value: PrimaryExpression::Identifier(object_identifier),
                }),
            }
        });

        let tuple = expr
            .clone()
            .separated_by(just(Token::Comma))
            .at_least(2)
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .map_with(|values, e| tuple_expression_node(values, e.span()));

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

        let list = expr
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LBracket), just(Token::RBracket))
            .map_with(|items, e| Node {
                pos: pos_from_span_source(e.span()),
                value: Expression::Bracket(Node {
                    pos: pos_from_span_source(e.span()),
                    value: BracketExpression {
                        content: BracketContent::ListLiteral(items),
                    },
                }),
            });

        let atom = choice((literal, tuple, grouped, list, ident)).boxed();

        let invoke_segment = args_tuple
            .clone()
            .map(|args| CallChainSegment::Invoke { args });

        let field_segment = just(Token::Dot).ignore_then(choice((
            select! { Token::IntLiteral(index) => index }
                .try_map(|index, span| {
                    index.parse::<usize>().map_err(|_| {
                        Rich::custom(span, format!("tuple index '{}' is too large", index))
                    })
                })
                .map(|index| CallChainSegment::TupleIndex { index }),
            select! { Token::Ident(name) => name }
                .then(args_tuple.clone().or_not())
                .map_with(|(name, args), e| {
                    let name = Node {
                        pos: pos_from_span_source(e.span()),
                        value: Identifier { value: name },
                    };
                    match args {
                        Some(args) => CallChainSegment::Call { name, args },
                        None => CallChainSegment::Field { name },
                    }
                }),
        )));

        let postfix = atom
            .then(
                choice((invoke_segment, field_segment))
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map_with(|(base, segments), e| {
                if segments.is_empty() {
                    base
                } else {
                    Node {
                        pos: pos_from_span_source(e.span()),
                        value: Expression::CallChain(Node {
                            pos: pos_from_span_source(e.span()),
                            value: CallChainExpression {
                                base: Box::new(base),
                                segments,
                            },
                        }),
                    }
                }
            });

        postfix.pratt((
            prefix(7, just(Token::Bang), |_, rhs, e| {
                unary_expression_node(UnaryOperator::Not, rhs, e.span())
            }),
            prefix(7, just(Token::Plus), |_, rhs, e| {
                unary_expression_node(UnaryOperator::Positive, rhs, e.span())
            }),
            prefix(7, just(Token::Minus), |_, rhs, e| {
                unary_expression_node(UnaryOperator::Negative, rhs, e.span())
            }),
            infix(left(6), just(Token::Star), |l, _, r, e| {
                binary_expression_node(BinaryOperator::Multiply, l, r, e.span())
            }),
            infix(left(6), just(Token::Slash), |l, _, r, e| {
                binary_expression_node(BinaryOperator::Divide, l, r, e.span())
            }),
            infix(left(6), just(Token::Percent), |l, _, r, e| {
                binary_expression_node(BinaryOperator::Modulo, l, r, e.span())
            }),
            infix(left(5), just(Token::Plus), |l, _, r, e| {
                binary_expression_node(BinaryOperator::Add, l, r, e.span())
            }),
            infix(left(5), just(Token::Minus), |l, _, r, e| {
                binary_expression_node(BinaryOperator::Subtract, l, r, e.span())
            }),
            infix(left(4), just(Token::ShiftLeft), |l, _, r, e| {
                binary_expression_node(BinaryOperator::ShiftLeft, l, r, e.span())
            }),
            infix(left(4), just(Token::ShiftRight), |l, _, r, e| {
                binary_expression_node(BinaryOperator::ShiftRight, l, r, e.span())
            }),
            infix(left(3), just(Token::LtEq), |l, _, r, e| {
                binary_expression_node(BinaryOperator::LessThanOrEqual, l, r, e.span())
            }),
            infix(left(3), just(Token::GtEq), |l, _, r, e| {
                binary_expression_node(BinaryOperator::GreaterThanOrEqual, l, r, e.span())
            }),
            infix(left(3), just(Token::Lt), |l, _, r, e| {
                binary_expression_node(BinaryOperator::LessThan, l, r, e.span())
            }),
            infix(left(3), just(Token::Gt), |l, _, r, e| {
                binary_expression_node(BinaryOperator::GreaterThan, l, r, e.span())
            }),
            infix(left(2), just(Token::EqEq), |l, _, r, e| {
                binary_expression_node(BinaryOperator::Equals, l, r, e.span())
            }),
            infix(left(2), just(Token::NotEq), |l, _, r, e| {
                binary_expression_node(BinaryOperator::NotEquals, l, r, e.span())
            }),
            infix(left(1), just(Token::Amp), |l, _, r, e| {
                binary_expression_node(BinaryOperator::BitAnd, l, r, e.span())
            }),
            infix(left(1), just(Token::Pipe), |l, _, r, e| {
                binary_expression_node(BinaryOperator::BitOr, l, r, e.span())
            }),
            infix(left(0), just(Token::AndAnd), |l, _, r, e| {
                binary_expression_node(BinaryOperator::And, l, r, e.span())
            }),
            infix(left(0), just(Token::OrOr), |l, _, r, e| {
                binary_expression_node(BinaryOperator::Or, l, r, e.span())
            }),
        ))
    })
}

fn unsupported_block_parser<'tokens, I>(
    token: Token,
    block_name: &'static str,
) -> impl Parser<'tokens, I, TopLevelBlock, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    just(token)
        .map_with(move |_, e| e.span())
        .try_map(move |span, _| {
            Err(Rich::custom(
                span,
                format!("{block_name} blocks are not implemented yet"),
            ))
        })
}

fn unsupported_prefixed_block_parser<'tokens, I>(
    token: Token,
    block_name: &'static str,
) -> impl Parser<'tokens, I, TopLevelBlock, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    just(Token::AtPrivate)
        .or_not()
        .ignore_then(just(token))
        .map_with(move |_, e| e.span())
        .try_map(move |span, _| {
            Err(Rich::custom(
                span,
                format!("{block_name} blocks are not implemented yet"),
            ))
        })
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

fn tuple_expression_node(values: Vec<Node<Expression>>, span: Span) -> Node<Expression> {
    Node {
        pos: pos_from_span_source(span),
        value: Expression::Tuple(Node {
            pos: pos_from_span_source(span),
            value: TupleExpression { values },
        }),
    }
}

fn unary_expression_node(
    operator: UnaryOperator,
    operand: Node<Expression>,
    span: Span,
) -> Node<Expression> {
    Node {
        pos: pos_from_span_source(span),
        value: Expression::Unary(Node {
            pos: pos_from_span_source(span),
            value: UnaryExpression {
                operator: Node {
                    pos: pos_from_span_source(span),
                    value: operator,
                },
                operand: Box::new(operand),
            },
        }),
    }
}

fn binary_expression_node(
    operator: BinaryOperator,
    left: Node<Expression>,
    right: Node<Expression>,
    span: Span,
) -> Node<Expression> {
    Node {
        pos: pos_from_span_source(span),
        value: Expression::Binary(Node {
            pos: pos_from_span_source(span),
            value: BinaryExpression {
                left: Box::new(left),
                operator: Node {
                    pos: pos_from_span_source(span),
                    value: operator,
                },
                right: Box::new(right),
            },
        }),
    }
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

fn map_rich_errors(
    source: &str,
    file_path: &str,
    errs: Vec<Rich<'_, Token, Span>>,
) -> AlthreadError {
    let err = errs.into_iter().next().expect("chumsky returned no errors");
    let message = format_rich_reason(&err);
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

fn format_rich_reason(err: &Rich<'_, Token, Span>) -> String {
    match err.reason() {
        RichReason::Custom(message) => message.clone(),
        RichReason::ExpectedFound { .. } => {
            let expected = err.expected().collect::<Vec<_>>();
            let expected_text = format_expected_patterns(&expected);
            let found_text = format_found_token(err.found());
            format!("expected {expected_text}, found {found_text}")
        }
    }
}

fn format_expected_patterns(expected: &[&RichPattern<'_, Token>]) -> String {
    if expected.iter().all(|pattern| is_datatype_pattern(pattern)) && !expected.is_empty() {
        return "a datatype".to_string();
    }

    let mut labels = expected
        .iter()
        .map(|pattern| format_pattern(pattern))
        .collect::<Vec<_>>();
    labels.sort();
    labels.dedup();

    match labels.as_slice() {
        [] => "something else".to_string(),
        [single] => single.clone(),
        [left, right] => format!("{left} or {right}"),
        many => {
            let mut out = many[..many.len() - 1].join(", ");
            out.push_str(", or ");
            out.push_str(&many[many.len() - 1]);
            out
        }
    }
}

fn format_pattern(pattern: &RichPattern<'_, Token>) -> String {
    match pattern {
        RichPattern::Token(token) => format!("'{}'", &**token),
        RichPattern::Label(label) => label.to_string(),
        RichPattern::Identifier(identifier) => format!("'{identifier}'"),
        RichPattern::Any => "any token".to_string(),
        RichPattern::SomethingElse => "something else".to_string(),
        RichPattern::EndOfInput => "end of input".to_string(),
        _ => "something".to_string(),
    }
}

fn format_found_token(found: Option<&Token>) -> String {
    match found {
        Some(token) => format!("'{}'", token),
        None => "end of input".to_string(),
    }
}

fn is_datatype_pattern(pattern: &RichPattern<'_, Token>) -> bool {
    matches!(
        pattern,
        RichPattern::Token(token)
            if matches!(
                &**token,
                Token::BoolType
                    | Token::IntType
                    | Token::FloatType
                    | Token::StringType
                    | Token::VoidType
                    | Token::Proc
                    | Token::List
                    | Token::Tuple
            )
    )
}
