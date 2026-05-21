use chumsky::{
    error::{RichPattern, RichReason},
    extra,
    input::ValueInput,
    pratt::*,
    prelude::*,
};
use ordered_float::OrderedFloat;
use std::{cell::RefCell, thread_local};

use crate::{
    ast::import_block::{ImportBlock, ImportItem, ImportPath},
    ast::statement::expression::list_expression::RangeListExpression,
    ast::{
        block::Block,
        condition_block::ConditionBlock,
        node::Node,
        statement::{
            assignment::{binary_assignment::BinaryAssignment, Assignment},
            atomic::Atomic,
            break_loop::{BreakLoopControl, BreakLoopType},
            channel_declaration::ChannelDeclaration,
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
            label::LabelStatement,
            loop_control::LoopControl,
            receive::ReceiveStatement,
            run_call::RunCall,
            send::SendStatement,
            wait::{Wait, WaitingBlockKind},
            waiting_case::{WaitingBlockCase, WaitingBlockCaseRule},
            while_control::WhileControl,
            Statement,
        },
        token::{
            args_list::ArgsList, binary_assignment_operator::BinaryAssignmentOperator,
            binary_operator::BinaryOperator, condition_keyword::ConditionKeyword,
            datatype::DataType, declaration_keyword::DeclarationKeyword, identifier::Identifier,
            literal::Literal, null_identifier::NullIdentifier,
            object_identifier::ObjectIdentifier, tuple_identifier::{Lvalue, TupleIdentifier},
            unary_operator::UnaryOperator,
        },
        Ast,
    },
    checker::ltl::ast::{CheckBlock, LtlExpression},
    error::{AlthreadError, ErrorType, Pos},
    parser::{
        lexer::{lex, lex_snippet, Span, Token},
        syntax::SyntaxSnippet,
    },
};

pub(crate) type ParserExtra<'a> = extra::Err<Rich<'a, Token, Span>>;

#[derive(Clone, Debug)]
struct ParserPosContext {
    file_path: String,
}

thread_local! {
    static PARSER_POS_CONTEXT: RefCell<Option<ParserPosContext>> = const { RefCell::new(None) };
}

#[derive(Debug)]
enum TopLevelBlock {
    Import(Node<ImportBlock>),
    Shared(Node<Block>),
    Main(Node<Block>),
    Always(Node<ConditionBlock>),
    Check {
        pos: Pos,
        body_span: Span,
    },
    Program {
        name: Node<Identifier>,
        args: Node<ArgsList>,
        block: Node<Block>,
        is_private: bool,
    },
    Function {
        name: Node<Identifier>,
        args: Node<ArgsList>,
        return_type: Node<DataType>,
        block: Node<Block>,
        is_private: bool,
    },
}

pub fn parse_program(source: &str, file_path: &str) -> Result<Ast, AlthreadError> {
    let blocks = with_parser_pos_context(file_path, || {
        let tokens = lex(source, file_path)?;
        let eoi = Span::new((), source.len()..source.len());
        let parser = ast_parser(source, file_path);
        let input = tokens.as_slice().map(eoi, |(token, span)| (token, span));

        parser
            .parse(input)
            .into_result()
            .map_err(|errs| map_rich_errors(source, file_path, errs))
    })?;

    let mut ast = Ast::new();
    for block in blocks {
        match block {
            TopLevelBlock::Import(import) => ast.import_block = Some(import),
            TopLevelBlock::Shared(block) => ast.global_block = Some(block),
            TopLevelBlock::Main(block) => {
                ast.process_blocks
                    .insert("main".to_string(), (Node::new(), block, false));
            }
            TopLevelBlock::Always(block) => {
                ast.condition_blocks.insert(ConditionKeyword::Always, block);
            }
            TopLevelBlock::Check { pos, body_span } => {
                let formulas = parse_check_formulas(source, file_path, body_span)?;
                ast.check_blocks.push(Node {
                    pos,
                    value: CheckBlock { formulas },
                });
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
            TopLevelBlock::Function {
                name,
                args,
                return_type,
                block,
                is_private,
            } => {
                ast.function_blocks.insert(
                    name.value.value.clone(),
                    (args, return_type.value, block, is_private),
                );
            }
        }
    }

    Ok(ast)
}

pub fn parse_ast(source: &str, file_path: &str) -> Result<Ast, AlthreadError> {
    parse_program(source, file_path)
}

pub fn parse_datatype(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<DataType>, AlthreadError> {
    with_parser_pos_context(file_path, || {
        let tokens = lex_snippet(snippet, file_path)?;
        let eoi = Span::new((), snippet.pos.end..snippet.pos.end);
        let parser = datatype_parser();
        let input = tokens.as_slice().map(eoi, |(token, span)| (token, span));
        parser
            .parse(input)
            .into_result()
            .map_err(|errs| map_rich_errors(source, file_path, errs))
    })
}

pub fn parse_object_identifier(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<ObjectIdentifier>, AlthreadError> {
    with_parser_pos_context(file_path, || {
        let tokens = lex_snippet(snippet, file_path)?;
        let eoi = Span::new((), snippet.pos.end..snippet.pos.end);
        let parser = object_identifier_parser();
        let input = tokens.as_slice().map(eoi, |(token, span)| (token, span));
        parser
            .parse(input)
            .into_result()
            .map_err(|errs| map_rich_errors(source, file_path, errs))
    })
}

pub fn parse_expression(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<Expression>, AlthreadError> {
    with_parser_pos_context(file_path, || {
        let tokens = lex_snippet(snippet, file_path)?;
        let eoi = Span::new((), snippet.pos.end..snippet.pos.end);
        let parser = expression_parser();
        let input = tokens.as_slice().map(eoi, |(token, span)| (token, span));
        parser
            .parse(input)
            .into_result()
            .map_err(|errs| map_rich_errors(source, file_path, errs))
    })
}

pub fn parse_list_expression(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<Expression>, AlthreadError> {
    parse_expression(source, snippet, file_path)
}

pub(crate) fn parse_ltl_expression_with_chumsky(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> Result<LtlExpression, AlthreadError> {
    with_parser_pos_context(filepath, || {
        let tokens = lex_snippet(snippet, filepath)?;
        let eoi = Span::new((), snippet.pos.end..snippet.pos.end);
        let input = tokens.as_slice().map(eoi, |(token, span)| (token, span));
        let result = ltl_expression_parser()
            .parse(input)
            .into_result()
            .map_err(|errs| map_ltl_errors(source, filepath, errs));
        result
    })
}

fn ast_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, Vec<TopLevelBlock>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    choice((
        import_block_parser(source, file_path).boxed(),
        shared_block_parser(source, file_path).boxed(),
        main_block_parser(source, file_path).boxed(),
        always_block_parser(source, file_path).boxed(),
        check_block_parser(source, file_path).boxed(),
        program_block_parser(source, file_path).boxed(),
        function_block_parser(source, file_path).boxed(),
    ))
    .repeated()
    .collect::<Vec<TopLevelBlock>>()
    .then_ignore(end())
}

fn import_block_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, TopLevelBlock, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    just(Token::Import)
        .ignore_then(
            import_entry_parser()
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        )
        .try_map(move |imports, span| {
            ImportBlock::validate_import_names(&imports)
                .map_err(|err| Rich::custom(span, err.message.clone()))?;

            Ok(TopLevelBlock::Import(Node {
                pos: pos_from_span(source, file_path, span),
                value: ImportBlock { imports },
            }))
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

fn always_block_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, TopLevelBlock, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    just(Token::Always)
        .ignore_then(
            expression_parser()
                .then_ignore(just(Token::Semi))
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LBrace), just(Token::RBrace))
                .map_with(move |children, e| Node {
                    pos: pos_from_span(source, file_path, e.span()),
                    value: ConditionBlock { children },
                }),
        )
        .map(TopLevelBlock::Always)
}

fn check_block_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, TopLevelBlock, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    just(Token::Check)
        .ignore_then(
            balanced_token_item_parser()
                .repeated()
                .map_with(|_, e| e.span())
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        )
        .map_with(move |body_span, e| TopLevelBlock::Check {
            pos: pos_from_span(source, file_path, e.span()),
            body_span,
        })
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

fn function_block_parser<'tokens, 'src: 'tokens, I>(
    source: &'src str,
    file_path: &'src str,
) -> impl Parser<'tokens, I, TopLevelBlock, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    just(Token::AtPrivate)
        .or_not()
        .then_ignore(just(Token::Fn))
        .then(identifier_parser())
        .then(args_list_parser())
        .then_ignore(just(Token::Arrow))
        .then(datatype_parser())
        .then(block_parser(source, file_path))
        .map(
            |((((private_marker, name), args), return_type), block)| TopLevelBlock::Function {
                name,
                args,
                return_type,
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

fn import_path_parser<'tokens, I>(
) -> impl Parser<'tokens, I, ImportPath, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    identifier_parser()
        .map_with(|node, _e| node.value.value)
        .separated_by(just(Token::Slash))
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|segments| ImportPath { segments })
}

fn import_entry_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<ImportItem>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    import_path_parser()
        .then(just(Token::As).ignore_then(identifier_parser()).or_not())
        .then_ignore(just(Token::Semi))
        .map_with(|(path, alias), e| Node {
            pos: pos_from_span_source(e.span()),
            value: ImportItem { path, alias },
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

        let return_statement = just(Token::Return)
            .ignore_then(expression_parser().or_not())
            .then_ignore(just(Token::Semi))
            .map_with(move |value, e| Node {
                pos: pos_from_span(source, file_path, e.span()),
                value: Statement::FnReturn(Node {
                    pos: pos_from_span(source, file_path, e.span()),
                    value: crate::ast::statement::fn_return::FnReturn {
                        value,
                        pos: pos_from_span(source, file_path, e.span()),
                    },
                }),
            });

        let label_statement = just(Token::Label)
            .ignore_then(identifier_parser())
            .then_ignore(just(Token::Semi))
            .map_with(move |name, e| Node {
                pos: pos_from_span(source, file_path, e.span()),
                value: Statement::Label(Node {
                    pos: pos_from_span(source, file_path, e.span()),
                    value: LabelStatement { name },
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
            return_statement.boxed(),
            label_statement.boxed(),
            send_statement_parser().boxed(),
            channel_declaration_parser().boxed(),
            declaration_statement_parser().boxed(),
            assignment_statement_parser().boxed(),
            expression_statement_parser().boxed(),
            nested_block.boxed(),
        ))
    })
}

fn balanced_token_item_parser<'tokens, I>(
) -> impl Parser<'tokens, I, (), ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    recursive(|item| {
        let plain = any()
            .filter(|tok| {
                !matches!(
                    tok,
                    Token::LParen
                        | Token::RParen
                        | Token::LBrace
                        | Token::RBrace
                        | Token::LBracket
                        | Token::RBracket
                )
            })
            .ignored();

        let paren = item
            .clone()
            .repeated()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .ignored();
        let brace = item
            .clone()
            .repeated()
            .delimited_by(just(Token::LBrace), just(Token::RBrace))
            .ignored();
        let bracket = item
            .clone()
            .repeated()
            .delimited_by(just(Token::LBracket), just(Token::RBracket))
            .ignored();

        choice((plain, paren, brace, bracket))
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
        .then(lvalue_parser())
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

fn lvalue_parser<'tokens, I>() -> impl Parser<'tokens, I, Lvalue, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    recursive(|lvalue| {
        let null = select! {
            Token::Ident(name) if name == "_" => ()
        }
        .map_with(|_, e| {
            Lvalue::NullIdentifier(Node {
                pos: pos_from_span_source(e.span()),
                value: NullIdentifier,
            })
        });

        let ident = identifier_parser().map(Lvalue::Identifier);

        let tuple = lvalue
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .at_least(1)
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .map_with(|values, e| {
                Lvalue::TupleIdentifier(Node {
                    pos: pos_from_span_source(e.span()),
                    value: TupleIdentifier {
                        value: values.into_iter().map(Box::new).collect(),
                    },
                })
            });

        choice((tuple, null, ident))
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

fn send_statement_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<Statement>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    let args_tuple = expression_parser()
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::LParen), just(Token::RParen))
        .map_with(|values, e| tuple_expression_node(values, e.span()));

    just(Token::Send)
        .ignore_then(send_target_parser())
        .then(args_tuple)
        .then_ignore(just(Token::Semi))
        .map_with(|((channel, is_broadcast), values), e| Node {
            pos: pos_from_span_source(e.span()),
            value: Statement::Send(Node {
                pos: pos_from_span_source(e.span()),
                value: SendStatement {
                    channel,
                    is_broadcast,
                    values,
                },
            }),
        })
}

fn channel_declaration_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<Statement>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    let endpoint = identifier_parser()
        .then_ignore(just(Token::Dot))
        .then(channel_path_parts_parser());

    just(Token::Channel)
        .ignore_then(endpoint.clone())
        .then(
            datatype_parser()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LParen), just(Token::RParen)),
        )
        .then_ignore(just(Token::Gt))
        .then(endpoint)
        .then_ignore(just(Token::Semi))
        .map_with(
            |(((left_prog, left_name), datatypes), (right_prog, right_name)), e| Node {
                pos: pos_from_span_source(e.span()),
                value: Statement::ChannelDeclaration(Node {
                    pos: pos_from_span_source(e.span()),
                    value: ChannelDeclaration {
                        ch_left_prog: left_prog,
                        ch_left_name: left_name
                            .into_iter()
                            .map(|node: Node<Identifier>| node.value.value)
                            .collect::<Vec<_>>()
                            .join("."),
                        ch_right_prog: right_prog,
                        ch_right_name: right_name
                            .into_iter()
                            .map(|node: Node<Identifier>| node.value.value)
                            .collect::<Vec<_>>()
                            .join("."),
                        datatypes: datatypes.into_iter().map(|dtype| dtype.value).collect(),
                    },
                }),
            },
        )
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
    select! {
        Token::Ident(name) => name,
        // other keywords that can be used as identifiers
        Token::Dollar => "$".to_string(),
        Token::First => "first".to_string(),
        Token::Seq => "seq".to_string(),

    }
    .labelled("identifier")
    .map_with(|name, e| Node {
        pos: pos_from_span_source(e.span()),
        value: Identifier { value: name },
    })
}

fn literal_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<Literal>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    fn parse_int_literal(value: &str) -> i64 {
        if let Some(hex) = value
            .strip_prefix("0x")
            .or_else(|| value.strip_prefix("0X"))
        {
            i64::from_str_radix(hex, 16).expect("valid hex int literal")
        } else if let Some(bin) = value
            .strip_prefix("0b")
            .or_else(|| value.strip_prefix("0B"))
        {
            i64::from_str_radix(bin, 2).expect("valid binary int literal")
        } else {
            value.parse::<i64>().expect("valid int literal")
        }
    }

    select! {
        Token::True => Literal::Bool(true),
        Token::False => Literal::Bool(false),
        Token::Null => Literal::Null,
        Token::IntLiteral(value) => Literal::Int(parse_int_literal(&value)),
        Token::FloatLiteral(value) => Literal::Float(OrderedFloat(value.parse::<f64>().expect("valid float literal"))),
        Token::StringLiteral(value) => Literal::String(unquote_string(&value)),
    }
    .labelled("literal")
    .map_with(|value, e| Node {
        pos: pos_from_span_source(e.span()),
        value,
    })
}

pub(crate) fn expression_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<Expression>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    recursive(|expr| {
        let expr_block = expr
            .clone()
            .delimited_by(just(Token::LBrace), just(Token::RBrace));

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

        let if_expr = just(Token::If)
            .ignore_then(expr.clone())
            .then(expr_block.clone())
            .then(just(Token::Else).ignore_then(expr_block.clone()).or_not())
            .map_with(|((condition, then_expr), else_expr), e| Node {
                pos: pos_from_span_source(e.span()),
                value: Expression::Primary(Node {
                    pos: pos_from_span_source(e.span()),
                    value: PrimaryExpression::IfExpr {
                        condition: Box::new(condition),
                        then_expr: Box::new(then_expr),
                        else_expr: else_expr.map(Box::new),
                    },
                }),
            });

        let forall_expr = just(Token::For)
            .ignore_then(identifier_parser())
            .then_ignore(just(Token::In))
            .then(expr.clone())
            .then(expr_block.clone())
            .map_with(|((var, list), body), e| Node {
                pos: pos_from_span_source(e.span()),
                value: Expression::Primary(Node {
                    pos: pos_from_span_source(e.span()),
                    value: PrimaryExpression::ForAllExpr {
                        var,
                        list: Box::new(list),
                        body: Box::new(body),
                    },
                }),
            });

        let exists_expr = just(Token::Exists)
            .ignore_then(identifier_parser())
            .then_ignore(just(Token::In))
            .then(expr.clone())
            .then(expr_block.clone())
            .map_with(|((var, list), body), e| Node {
                pos: pos_from_span_source(e.span()),
                value: Expression::Primary(Node {
                    pos: pos_from_span_source(e.span()),
                    value: PrimaryExpression::ExistsExpr {
                        var,
                        list: Box::new(list),
                        body: Box::new(body),
                    },
                }),
            });

        let atom = choice((
            if_expr,
            forall_expr,
            exists_expr,
            run_call,
            literal,
            tuple,
            grouped,
            list,
            ident,
        ))
        .boxed();

        let invoke_segment = args_tuple
            .clone()
            .map(|args| CallChainSegment::Invoke { args });

        let field_segment = just(Token::Dot).ignore_then(choice((
            just(Token::Ident("reaches".to_string()))
                .ignore_then(
                    identifier_parser().delimited_by(just(Token::LParen), just(Token::RParen)),
                )
                .map(|label| CallChainSegment::Reaches { label }),
            select! { Token::IntLiteral(index) => index }
                .labelled("int literal for tuple index")
                .try_map(|index, span| {
                    index.parse::<usize>().map_err(|_| {
                        Rich::custom(span, format!("tuple index '{}' is too large", index))
                    })
                })
                .then(args_tuple.clone().or_not())
                .try_map(|(index, args), span| {
                    if let Some(args) = args {
                        let Expression::Tuple(tuple) = &args.value else {
                            unreachable!("argument tuples are always tuple expressions");
                        };
                        if !tuple.value.values.is_empty() {
                            return Err(Rich::custom(
                                span,
                                "tuple access '.N()' does not accept arguments",
                            ));
                        }
                    }
                    Ok(CallChainSegment::TupleIndex { index })
                }),
            identifier_parser()
                .then(args_tuple.clone().or_not())
                .map(|(name, args)| match args {
                    Some(args) => CallChainSegment::Call { name, args },
                    None => CallChainSegment::Field { name },
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

pub(crate) fn predicate_expression_parser<'tokens, I>(
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
            just(Token::Ident("reaches".to_string()))
                .ignore_then(
                    identifier_parser().delimited_by(just(Token::LParen), just(Token::RParen)),
                )
                .map(|label| CallChainSegment::Reaches { label }),
            select! { Token::IntLiteral(index) => index }
                .labelled("int literal for tuple index")
                .try_map(|index, span| {
                    index.parse::<usize>().map_err(|_| {
                        Rich::custom(span, format!("tuple index '{}' is too large", index))
                    })
                })
                .then(args_tuple.clone().or_not())
                .try_map(|(index, args), span| {
                    if let Some(args) = args {
                        let Expression::Tuple(tuple) = &args.value else {
                            unreachable!("argument tuples are always tuple expressions");
                        };
                        if !tuple.value.values.is_empty() {
                            return Err(Rich::custom(
                                span,
                                "tuple access '.N()' does not accept arguments",
                            ));
                        }
                    }
                    Ok(CallChainSegment::TupleIndex { index })
                }),
            identifier_parser()
                .then(args_tuple.clone().or_not())
                .map(|(name, args)| match args {
                    Some(args) => CallChainSegment::Call { name, args },
                    None => CallChainSegment::Field { name },
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

type LtlParserExtra<'a> = extra::Err<Rich<'a, Token, Span>>;

fn ltl_expression_parser<'tokens, I>(
) -> impl Parser<'tokens, I, LtlExpression, LtlParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    recursive(|ltl| {
        let predicate_expr = predicate_expression_parser().map(normalize_ltl_predicate_expression);
        let condition_formula = recursive(|cond| {
            let grouped = cond
                .clone()
                .delimited_by(just(Token::LParen), just(Token::RParen));
            let predicate = predicate_expr.clone().map(LtlExpression::Predicate);
            choice((grouped, predicate)).pratt((
                prefix(3, just(Token::Bang), |_, rhs, _| {
                    LtlExpression::Not(Box::new(rhs))
                }),
                infix(left(2), just(Token::AndAnd), |lhs, _, rhs, _| {
                    LtlExpression::And(Box::new(lhs), Box::new(rhs))
                }),
                infix(left(1), just(Token::OrOr), |lhs, _, rhs, _| {
                    LtlExpression::Or(Box::new(lhs), Box::new(rhs))
                }),
            ))
        });

        let ltl_block = ltl
            .clone()
            .then_ignore(just(Token::Semi).or_not())
            .delimited_by(just(Token::LBrace), just(Token::RBrace));

        let grouped = ltl
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen));

        let if_formula = just(Token::If)
            .ignore_then(condition_formula)
            .then(ltl_block.clone())
            .then(just(Token::Else).ignore_then(ltl_block.clone()).or_not())
            .map(|((condition, then_expr), else_expr)| {
                if let Some(else_expr) = else_expr {
                    let condition_box = Box::new(condition);
                    let implies_then =
                        LtlExpression::Implies(condition_box.clone(), Box::new(then_expr));
                    let implies_else = LtlExpression::Implies(
                        Box::new(LtlExpression::Not(condition_box)),
                        Box::new(else_expr),
                    );
                    LtlExpression::And(Box::new(implies_then), Box::new(implies_else))
                } else {
                    LtlExpression::Implies(Box::new(condition), Box::new(then_expr))
                }
            });

        let for_formula = just(Token::For)
            .ignore_then(identifier_parser())
            .then_ignore(just(Token::In))
            .then(predicate_expr.clone())
            .then(ltl_block.clone())
            .map(|((var_name, list), body)| LtlExpression::ForLoop {
                var_name: var_name.value.value,
                list,
                body: Box::new(body),
            });

        let predicate = predicate_expr.map(LtlExpression::Predicate);

        choice((if_formula, for_formula, grouped, predicate)).pratt((
            prefix(4, just(Token::Always), |_, rhs, _| {
                LtlExpression::Always(Box::new(rhs))
            }),
            prefix(4, just(Token::Eventually), |_, rhs, _| {
                LtlExpression::Eventually(Box::new(rhs))
            }),
            prefix(4, just(Token::Bang), |_, rhs, _| {
                LtlExpression::Not(Box::new(rhs))
            }),
            infix(right(3), just(Token::Until), |lhs, _, rhs, _| {
                LtlExpression::Until(Box::new(lhs), Box::new(rhs))
            }),
            infix(left(2), just(Token::AndAnd), |lhs, _, rhs, _| {
                LtlExpression::And(Box::new(lhs), Box::new(rhs))
            }),
            infix(left(1), just(Token::OrOr), |lhs, _, rhs, _| {
                LtlExpression::Or(Box::new(lhs), Box::new(rhs))
            }),
            infix(right(0), just(Token::Arrow), |lhs, _, rhs, _| {
                LtlExpression::Implies(Box::new(lhs), Box::new(rhs))
            }),
        ))
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

    let fn_name_pos = object_identifier_parts_pos(&parts).unwrap_or_else(|| expr_pos.clone());

    Ok(Node {
        pos: expr_pos.clone(),
        value: Statement::FnCall(Node {
            pos: expr_pos.clone(),
            value: FnCall {
                fn_name: Node {
                    pos: fn_name_pos,
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

fn object_identifier_parts_pos(parts: &[Node<Identifier>]) -> Option<Pos> {
    let first = parts.first()?;
    let last = parts.last()?;
    Some(Pos::new(
        first.pos.file_path.clone(),
        first.pos.start,
        last.pos.end,
    ))
}

fn receive_rule_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<ReceiveStatement>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    just(Token::Receive)
        .ignore_then(channel_path_string_parser())
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

fn send_target_parser<'tokens, I>(
) -> impl Parser<'tokens, I, (String, bool), ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    channel_path_string_parser()
        .then(
            just(Token::Dot)
                .ignore_then(just(Token::Star))
                .to(true)
                .or_not(),
        )
        .map_with(|(channel, is_broadcast), e| {
            let pos = pos_from_span_source(e.span());
            if channel.is_empty() {
                Err(Rich::custom(
                    span_from_pos(&pos),
                    "send target must include a channel name",
                ))
            } else {
                Ok((channel, is_broadcast.unwrap_or(false)))
            }
        })
        .try_map(|result, _| result)
}

fn channel_path_part_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Node<Identifier>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    choice((
        identifier_parser(),
        just(Token::In).map_with(|_, e| Node {
            pos: pos_from_span_source(e.span()),
            value: Identifier {
                value: "in".to_string(),
            },
        }),
    ))
}

fn channel_path_parts_parser<'tokens, I>(
) -> impl Parser<'tokens, I, Vec<Node<Identifier>>, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    channel_path_part_parser()
        .separated_by(just(Token::Dot))
        .at_least(1)
        .collect::<Vec<_>>()
}

fn channel_path_string_parser<'tokens, I>(
) -> impl Parser<'tokens, I, String, ParserExtra<'tokens>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    channel_path_parts_parser().map(|parts| {
        parts
            .into_iter()
            .map(|part| part.value.value)
            .collect::<Vec<_>>()
            .join(".")
    })
}

fn apply_atomic_prefix(statement: Node<Statement>, pos: Pos) -> Node<Statement> {
    let statement_pos = statement.pos.clone();
    let atomic_statement = match statement.value {
        Statement::Block(block) => Node {
            pos: statement_pos,
            value: Statement::Block(block),
        },
        other => Node {
            pos: statement_pos.clone(),
            value: Statement::Block(Node {
                pos: statement_pos,
                value: Block {
                    children: vec![Node {
                        pos: pos.clone(),
                        value: other,
                    }],
                },
            }),
        },
    };

    Node {
        pos: pos.clone(),
        value: Statement::Atomic(Node {
            pos: pos.clone(),
            value: Atomic {
                // Keep `atomic await ...` on the same compilation path as
                // `atomic { await ...; ... }`. That way the atomic region is
                // explicitly closed with `AtomicEnd`, and a later re-wait in a
                // loop cannot be merged into the same VM step.
                statement: Box::new(atomic_statement),
                delegated: false,
            },
        }),
    }
}

fn unquote_string(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(value)
        .to_string()
}

fn with_parser_pos_context<T>(
    file_path: &str,
    f: impl FnOnce() -> Result<T, AlthreadError>,
) -> Result<T, AlthreadError> {
    let previous = PARSER_POS_CONTEXT.with(|ctx| {
        ctx.replace(Some(ParserPosContext {
            file_path: file_path.to_string(),
        }))
    });

    let result = f();

    PARSER_POS_CONTEXT.with(|ctx| {
        let _ = ctx.replace(previous);
    });

    result
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
    PARSER_POS_CONTEXT.with(|ctx| {
        if let Some(ctx) = ctx.borrow().as_ref() {
            Pos::new(ctx.file_path.clone(), span.start, span.end)
        } else {
            Pos::new(String::new(), span.start, span.end)
        }
    })
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

fn map_ltl_errors(
    source: &str,
    file_path: &str,
    errs: Vec<Rich<'_, Token, Span>>,
) -> AlthreadError {
    let err = errs.into_iter().next().expect("chumsky returned no errors");
    let message = match err.reason() {
        RichReason::Custom(message) => message.clone(),
        RichReason::ExpectedFound { .. } => {
            let expected = err
                .expected()
                .map(|pattern| format_pattern(pattern))
                .collect::<Vec<_>>();
            let found = err
                .found()
                .map(|token| format!("'{}'", token))
                .unwrap_or_else(|| "end of input".to_string());
            if expected.is_empty() {
                format!("unexpected {found}")
            } else if expected.len() == 1 {
                format!("expected {}, found {found}", expected[0])
            } else {
                format!("expected one of {}, found {found}", expected.join(", "))
            }
        }
    };

    AlthreadError::new(
        ErrorType::SyntaxError,
        Some(Pos::from_offsets(
            source,
            file_path,
            err.span().start,
            err.span().end.max(err.span().start + 1),
        )),
        message,
    )
}

fn normalize_ltl_predicate_expression(expr: Node<Expression>) -> Node<Expression> {
    match expr.value {
        Expression::Primary(primary) => match primary.value {
            PrimaryExpression::Expression(inner) => *inner,
            _ => Node {
                pos: expr.pos,
                value: Expression::Primary(primary),
            },
        },
        _ => expr,
    }
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
        Some(token) => match token {
            Token::StringLiteral(value) => format!("\"{value}\""),
            Token::First
            | Token::Seq
            | Token::Await
            | Token::Atomic
            | Token::Receive
            | Token::Send
            | Token::In
            | Token::If
            | Token::Else
            | Token::For
            | Token::Loop
            | Token::Always
            | Token::Eventually
            | Token::Until => format!("keyword '{}'", token),
            Token::Proc
            | Token::List
            | Token::Tuple
            | Token::BoolType
            | Token::IntType
            | Token::FloatType
            | Token::StringType
            | Token::VoidType => format!("datatype '{}'", token),
            _ => format!("'{}'", token),
        },
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

fn parse_check_formulas(
    source: &str,
    file_path: &str,
    body_span: Span,
) -> Result<Vec<crate::checker::ltl::ast::LtlExpression>, AlthreadError> {
    let body = &source[body_span.start..body_span.end];
    let mut formulas = Vec::new();
    let mut formula_start = 0usize;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;

    for (idx, ch) in body.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ';' if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                if let Some(snippet) =
                    trimmed_snippet_from_offsets(source, file_path, body_span, formula_start, idx)
                {
                    formulas.push(parse_ltl_expression_with_chumsky(
                        source, &snippet, file_path,
                    )?);
                }
                formula_start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    if trimmed_snippet_from_offsets(source, file_path, body_span, formula_start, body.len())
        .is_some()
    {
        return Err(AlthreadError::new(
            ErrorType::SyntaxError,
            Some(Pos::from_offsets(
                source,
                file_path,
                body_span.start + formula_start,
                body_span.end,
            )),
            "expected ';' after LTL formula".to_string(),
        ));
    }

    Ok(formulas)
}

fn trimmed_snippet_from_offsets(
    source: &str,
    file_path: &str,
    body_span: Span,
    local_start: usize,
    local_end: usize,
) -> Option<SyntaxSnippet> {
    let text = &source[body_span.start + local_start..body_span.start + local_end];
    let leading_ws = text.len() - text.trim_start().len();
    let trailing_ws = text.len() - text.trim_end().len();
    let trimmed_start = local_start + leading_ws;
    let trimmed_end = local_end.saturating_sub(trailing_ws);

    if trimmed_start >= trimmed_end {
        return None;
    }

    let abs_start = body_span.start + trimmed_start;
    let abs_end = body_span.start + trimmed_end;
    Some(SyntaxSnippet::new(
        Pos::from_offsets(source, file_path, abs_start, abs_end),
        source[abs_start..abs_end].to_string(),
    ))
}
