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
    ast::statement::expression::list_expression::RangeListExpression,
    ast::{
        block::Block,
        node::Node,
        statement::{
            assignment::{binary_assignment::BinaryAssignment, Assignment},
            atomic::Atomic,
            break_loop::{BreakLoopControl, BreakLoopType},
            declaration::Declaration,
            expression::{
                binary_expression::BinaryExpression, primary_expression::PrimaryExpression,
                tuple_expression::TupleExpression, unary_expression::UnaryExpression,
                BracketContent, BracketExpression, CallChainExpression, CallChainSegment,
                Expression,
            },
            fn_call::FnCall,
            for_control::ForControl,
            if_control::IfControl,
            loop_control::LoopControl,
            receive::ReceiveStatement,
            run_call::RunCall,
            wait::{Wait, WaitingBlockKind},
            waiting_case::{WaitingBlockCase, WaitingBlockCaseRule},
            while_control::WhileControl,
            Statement,
        },
        token::{
            args_list::ArgsList, binary_assignment_operator::BinaryAssignmentOperator,
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
    #[token("@")]
    At,
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
    #[token("run")]
    Run,
    #[token("await")]
    Await,
    #[token("first")]
    First,
    #[token("seq")]
    Seq,
    #[token("receive")]
    Receive,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("loop")]
    Loop,
    #[token("atomic")]
    Atomic,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
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
    #[token("=>")]
    FatArrow,
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
    #[token("..")]
    DotDot,
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
            Token::At => write!(f, "@"),
            Token::Shared => write!(f, "shared"),
            Token::Main => write!(f, "main"),
            Token::Always => write!(f, "always"),
            Token::Never => write!(f, "never"),
            Token::Check => write!(f, "check"),
            Token::Program => write!(f, "program"),
            Token::Fn => write!(f, "fn"),
            Token::Let => write!(f, "let"),
            Token::Const => write!(f, "const"),
            Token::Run => write!(f, "run"),
            Token::Await => write!(f, "await"),
            Token::First => write!(f, "first"),
            Token::Seq => write!(f, "seq"),
            Token::Receive => write!(f, "receive"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::While => write!(f, "while"),
            Token::For => write!(f, "for"),
            Token::In => write!(f, "in"),
            Token::Loop => write!(f, "loop"),
            Token::Atomic => write!(f, "atomic"),
            Token::Break => write!(f, "break"),
            Token::Continue => write!(f, "continue"),
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
            Token::FatArrow => write!(f, "=>"),
            Token::EqEq => write!(f, "=="),
            Token::NotEq => write!(f, "!="),
            Token::LtEq => write!(f, "<="),
            Token::GtEq => write!(f, ">="),
            Token::ShiftLeft => write!(f, "<<"),
            Token::ShiftRight => write!(f, ">>"),
            Token::DotDot => write!(f, ".."),
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
    Main(Node<Block>),
    Program {
        name: Node<Identifier>,
        args: Node<ArgsList>,
        block: Node<Block>,
        is_private: bool,
    },
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
        main_block_parser(source, file_path).boxed(),
        unsupported_block_parser(Token::Always, "always").boxed(),
        unsupported_block_parser(Token::Never, "never").boxed(),
        unsupported_block_parser(Token::Check, "check").boxed(),
        program_block_parser(source, file_path).boxed(),
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
                TopLevelBlock::Main(block) => {
                    ast.process_blocks
                        .insert("main".to_string(), (Node::new(), block, false));
                }
                TopLevelBlock::Program {
                    name,
                    args,
                    block,
                    is_private,
                } => {
                    ast.process_blocks
                        .insert(name.value.value.clone(), (args, block, is_private));
                }
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

fn main_block_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, TopLevelBlock, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    just(Token::Main)
        .ignore_then(block_parser(source, file_path))
        .map(TopLevelBlock::Main)
}

fn program_block_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, TopLevelBlock, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    just(Token::AtPrivate)
        .or_not()
        .then_ignore(just(Token::Program))
        .then(identifier_parser())
        .then(args_list_parser())
        .then(block_parser(source, file_path))
        .map(
            |(((private_marker, name), args), block)| TopLevelBlock::Program {
                name,
                args,
                block,
                is_private: private_marker.is_some(),
            },
        )
}

fn args_list_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<ArgsList>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    identifier_parser()
        .then_ignore(just(Token::Colon))
        .then(datatype_parser())
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::LParen), just(Token::RParen))
        .map_with(|args, e| {
            let (identifiers, datatypes): (Vec<_>, Vec<_>) = args.into_iter().unzip();
            Node {
                pos: pos_from_span_source(e.span()),
                value: ArgsList {
                    identifiers,
                    datatypes,
                },
            }
        })
}

fn block_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, Node<Block>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    statement_parser(source, file_path)
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::LBrace), just(Token::RBrace))
        .map_with(move |children, e| Node {
            pos: pos_from_span(source, file_path, e.span()),
            value: Block { children },
        })
}

fn statement_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, Node<Statement>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    recursive(move |statement| {
        let block = statement
            .clone()
            .repeated()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LBrace), just(Token::RBrace))
            .map_with(move |children, e| Node {
                pos: pos_from_span(source, file_path, e.span()),
                value: Block { children },
            });

        let nested_block = block.clone().map_with(move |block: Node<Block>, e| Node {
            pos: pos_from_span(source, file_path, e.span()),
            value: Statement::Block(Node {
                pos: pos_from_span(source, file_path, e.span()),
                value: block.value,
            }),
        });

        let if_statement = recursive(|if_statement| {
            just(Token::If)
                .ignore_then(expression_parser())
                .then(block.clone())
                .then(
                    just(Token::Else)
                        .ignore_then(block.clone().or(if_statement.map_with(
                            move |statement, e| Node {
                                pos: pos_from_span(source, file_path, e.span()),
                                value: Block {
                                    children: vec![statement],
                                },
                            },
                        )))
                        .or_not(),
                )
                .map_with(move |((condition, then_block), else_block), e| Node {
                    pos: pos_from_span(source, file_path, e.span()),
                    value: Statement::If(Node {
                        pos: pos_from_span(source, file_path, e.span()),
                        value: IfControl {
                            condition,
                            then_block: Box::new(then_block),
                            else_block: else_block.map(Box::new),
                        },
                    }),
                })
        });

        let while_statement = just(Token::While)
            .ignore_then(expression_parser())
            .then(block.clone())
            .map_with(move |(condition, then_block), e| Node {
                pos: pos_from_span(source, file_path, e.span()),
                value: Statement::While(Node {
                    pos: pos_from_span(source, file_path, e.span()),
                    value: WhileControl {
                        condition,
                        then_block: Box::new(then_block),
                    },
                }),
            });

        let for_statement = just(Token::For)
            .ignore_then(identifier_parser())
            .then_ignore(just(Token::In))
            .then(expression_parser())
            .then(statement.clone())
            .map_with(move |((identifier, expression), body), e| Node {
                pos: pos_from_span(source, file_path, e.span()),
                value: Statement::For(Node {
                    pos: pos_from_span(source, file_path, e.span()),
                    value: ForControl {
                        identifier,
                        expression,
                        statement: Box::new(body),
                    },
                }),
            });

        let loop_statement =
            just(Token::Loop)
                .ignore_then(statement.clone())
                .map_with(move |body, e| Node {
                    pos: pos_from_span(source, file_path, e.span()),
                    value: Statement::Loop(Node {
                        pos: pos_from_span(source, file_path, e.span()),
                        value: LoopControl {
                            statement: Box::new(body),
                        },
                    }),
                });

        let wait_statement =
            wait_statement_parser(source, file_path, statement.clone(), block.clone());

        let atomic_statement = choice((just(Token::Atomic), just(Token::At)))
            .ignore_then(statement.clone())
            .map_with(move |body, e| {
                apply_atomic_prefix(body, pos_from_span(source, file_path, e.span()))
            });

        let break_statement =
            just(Token::Break)
                .then_ignore(just(Token::Semi))
                .map_with(move |_, e| Node {
                    pos: pos_from_span(source, file_path, e.span()),
                    value: Statement::BreakLoop(Node {
                        pos: pos_from_span(source, file_path, e.span()),
                        value: BreakLoopControl {
                            kind: BreakLoopType::Break,
                            label: None,
                        },
                    }),
                });

        let continue_statement = just(Token::Continue)
            .then_ignore(just(Token::Semi))
            .map_with(move |_, e| Node {
                pos: pos_from_span(source, file_path, e.span()),
                value: Statement::BreakLoop(Node {
                    pos: pos_from_span(source, file_path, e.span()),
                    value: BreakLoopControl {
                        kind: BreakLoopType::Continue,
                        label: None,
                    },
                }),
            });

        choice((
            if_statement.boxed(),
            while_statement.boxed(),
            for_statement.boxed(),
            loop_statement.boxed(),
            atomic_statement.boxed(),
            wait_statement.boxed(),
            break_statement.boxed(),
            continue_statement.boxed(),
            declaration_statement_parser().boxed(),
            assignment_statement_parser().boxed(),
            expression_statement_parser().boxed(),
            nested_block.boxed(),
        ))
    })
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

fn assignment_statement_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<Statement>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    object_identifier_parser()
        .then_ignore(just(Token::Eq))
        .then(expression_parser())
        .then_ignore(just(Token::Semi))
        .map_with(|(identifier, value), e| {
            let pos = pos_from_span_source(e.span());
            Node {
                pos: pos.clone(),
                value: Statement::Assignment(Node {
                    pos: pos.clone(),
                    value: Assignment::Binary(Node {
                        pos: pos.clone(),
                        value: BinaryAssignment {
                            identifier,
                            operator: Node {
                                pos: pos.clone(),
                                value: BinaryAssignmentOperator::Assign,
                            },
                            value,
                        },
                    }),
                }),
            }
        })
}

fn expression_statement_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<Statement>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    expression_parser()
        .then_ignore(just(Token::Semi))
        .try_map(|expression, _| expression_to_statement(expression))
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

        let run_call = just(Token::Run)
            .ignore_then(object_identifier_parser())
            .then(args_tuple.clone())
            .map_with(|(identifier, args), e| Node {
                pos: pos_from_span_source(e.span()),
                value: Expression::RunCall(Box::new(Node {
                    pos: pos_from_span_source(e.span()),
                    value: RunCall { identifier, args },
                })),
            });

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

        let atom = choice((run_call, literal, tuple, grouped, list, ident)).boxed();

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

        let non_range = postfix.pratt((
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
        ));

        non_range
            .clone()
            .then(just(Token::DotDot).ignore_then(expr.clone()).or_not())
            .map_with(|(start, end), e| match end {
                Some(end) => Node {
                    pos: pos_from_span_source(e.span()),
                    value: Expression::Range(Node {
                        pos: pos_from_span_source(e.span()),
                        value: RangeListExpression {
                            expression_start: Box::new(start),
                            expression_end: Box::new(end),
                        },
                    }),
                },
                None => start,
            })
    })
}

fn expression_to_statement(
    expression: Node<Expression>,
) -> Result<Node<Statement>, Rich<'static, Token, Span>> {
    match expression.value {
        Expression::RunCall(node) => Ok(Node {
            pos: expression.pos,
            value: Statement::Run(*node),
        }),
        Expression::FnCall(node) => Ok(Node {
            pos: expression.pos,
            value: Statement::FnCall(node),
        }),
        Expression::CallChain(node) => call_chain_to_statement(expression.pos, node),
        _ => Err(Rich::custom(
            span_from_pos(&expression.pos),
            "only function, method, and run calls can be used as standalone statements",
        )),
    }
}

fn call_chain_to_statement(
    expr_pos: Pos,
    chain: Node<CallChainExpression>,
) -> Result<Node<Statement>, Rich<'static, Token, Span>> {
    let mut parts = match &chain.value.base.value {
        Expression::Primary(primary) => match &primary.value {
            PrimaryExpression::Identifier(identifier) => identifier.value.parts.clone(),
            _ => {
                return Err(Rich::custom(
                    span_from_pos(&chain.pos),
                    "call statements must start from an identifier or object path",
                ));
            }
        },
        _ => {
            return Err(Rich::custom(
                span_from_pos(&chain.pos),
                "call statements must start from an identifier or object path",
            ));
        }
    };

    let mut final_args = None;
    for segment in chain.value.segments {
        match segment {
            CallChainSegment::Field { name } => {
                if final_args.is_some() {
                    return Err(Rich::custom(
                        span_from_pos(&expr_pos),
                        "chained statement calls after an invocation are not supported yet",
                    ));
                }
                parts.push(name);
            }
            CallChainSegment::Call { name, args } => {
                if final_args.is_some() {
                    return Err(Rich::custom(
                        span_from_pos(&expr_pos),
                        "chained statement calls after an invocation are not supported yet",
                    ));
                }
                parts.push(name);
                final_args = Some(args);
            }
            CallChainSegment::Invoke { args } => {
                if final_args.is_some() {
                    return Err(Rich::custom(
                        span_from_pos(&expr_pos),
                        "chained statement calls after an invocation are not supported yet",
                    ));
                }
                final_args = Some(args);
            }
            CallChainSegment::TupleIndex { .. } => {
                return Err(Rich::custom(
                    span_from_pos(&expr_pos),
                    "tuple indexing cannot be used as a standalone statement",
                ));
            }
            CallChainSegment::Reaches { .. } => {
                return Err(Rich::custom(
                    span_from_pos(&expr_pos),
                    "'reaches' predicates cannot be used as standalone statements",
                ));
            }
        }
    }

    let Some(values) = final_args else {
        return Err(Rich::custom(
            span_from_pos(&expr_pos),
            "only function, method, and run calls can be used as standalone statements",
        ));
    };

    Ok(Node {
        pos: expr_pos.clone(),
        value: Statement::FnCall(Node {
            pos: expr_pos.clone(),
            value: FnCall {
                fn_name: Node {
                    pos: expr_pos.clone(),
                    value: ObjectIdentifier { parts },
                },
                values: Box::new(values),
            },
        }),
    })
}

fn wait_statement_parser<'tokens, 'src: 'tokens, I, S, B>(
    source: &'src str,
    file_path: &'src str,
    statement: S,
    block: B,
) -> impl Parser<'tokens, I, Node<Statement>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
    S: Parser<'tokens, I, Node<Statement>, ParserExtra<'tokens>> + Clone + 'tokens,
    B: Parser<'tokens, I, Node<Block>, ParserExtra<'tokens>> + Clone + 'tokens,
{
    let receive_rule = receive_rule_parser();
    let receive_case = receive_rule
        .clone()
        .then(
            just(Token::FatArrow)
                .ignore_then(statement.clone())
                .or_not(),
        )
        .map_with(move |(rule, statement), e| Node {
            pos: pos_from_span(source, file_path, e.span()),
            value: WaitingBlockCase {
                rule: WaitingBlockCaseRule::Receive(rule),
                statement,
            },
        });

    let expression_case = expression_parser()
        .then(
            just(Token::FatArrow)
                .ignore_then(statement.clone())
                .or_not(),
        )
        .map_with(move |(rule, statement), e| Node {
            pos: pos_from_span(source, file_path, e.span()),
            value: WaitingBlockCase {
                rule: WaitingBlockCaseRule::Expression(rule),
                statement,
            },
        });

    let wait_block = choice((
        just(Token::First).to(WaitingBlockKind::First),
        just(Token::Seq).to(WaitingBlockKind::Seq),
    ))
    .then(
        choice((receive_case.boxed(), expression_case.boxed()))
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LBrace), just(Token::RBrace)),
    )
    .map_with(move |(block_kind, waiting_cases), e| Node {
        pos: pos_from_span(source, file_path, e.span()),
        value: Statement::Wait(Node {
            pos: pos_from_span(source, file_path, e.span()),
            value: Wait {
                block_kind,
                waiting_cases,
                start_atomic: false,
            },
        }),
    });

    let wait_single_receive = receive_rule
        .then(just(Token::FatArrow).ignore_then(statement).or_not())
        .then_ignore(just(Token::Semi).or_not())
        .map_with(move |(rule, statement), e| Node {
            pos: pos_from_span(source, file_path, e.span()),
            value: Statement::Wait(Node {
                pos: pos_from_span(source, file_path, e.span()),
                value: Wait {
                    block_kind: WaitingBlockKind::First,
                    waiting_cases: vec![Node {
                        pos: pos_from_span(source, file_path, e.span()),
                        value: WaitingBlockCase {
                            rule: WaitingBlockCaseRule::Receive(rule),
                            statement,
                        },
                    }],
                    start_atomic: false,
                },
            }),
        });

    let wait_single_expression =
        expression_parser()
            .then_ignore(just(Token::Semi))
            .map_with(move |rule, e| Node {
                pos: pos_from_span(source, file_path, e.span()),
                value: Statement::Wait(Node {
                    pos: pos_from_span(source, file_path, e.span()),
                    value: Wait {
                        block_kind: WaitingBlockKind::First,
                        waiting_cases: vec![Node {
                            pos: pos_from_span(source, file_path, e.span()),
                            value: WaitingBlockCase {
                                rule: WaitingBlockCaseRule::Expression(rule),
                                statement: None,
                            },
                        }],
                        start_atomic: false,
                    },
                }),
            });

    just(Token::Await).ignore_then(choice((
        wait_block.boxed(),
        wait_single_receive.boxed(),
        wait_single_expression.boxed(),
        block
            .clone()
            .ignored()
            .try_map(move |_, span| Err(Rich::custom(span, "bare await blocks are not supported"))),
    )))
}

fn receive_rule_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<ReceiveStatement>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    let channel_name = choice((
        select! { Token::Ident(name) => name },
        just(Token::In).to("in".to_string()),
    ));

    just(Token::Receive)
        .ignore_then(channel_name)
        .then(
            identifier_parser()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LParen), just(Token::RParen)),
        )
        .map_with(|(channel, variables), e| Node {
            pos: pos_from_span_source(e.span()),
            value: ReceiveStatement {
                channel,
                variables: variables.into_iter().map(|var| var.value.value).collect(),
            },
        })
}

fn apply_atomic_prefix(statement: Node<Statement>, pos: Pos) -> Node<Statement> {
    match statement.value {
        Statement::Wait(mut wait) => {
            wait.pos = pos.clone();
            wait.value.start_atomic = true;
            Node {
                pos,
                value: Statement::Wait(wait),
            }
        }
        other => Node {
            pos: pos.clone(),
            value: Statement::Atomic(Node {
                pos,
                value: Atomic {
                    statement: Box::new(Node {
                        pos: statement.pos,
                        value: other,
                    }),
                    delegated: false,
                },
            }),
        },
    }
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

fn span_from_pos(pos: &Pos) -> Span {
    Span::new((), pos.start..pos.end)
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
