use chumsky::{
    error::Simple,
    extra,
    prelude::{custom, end, Parser},
    span::{SimpleSpan, Span},
};

use crate::{
    ast::{
        import_block::{ImportBlock, ImportItem, ImportPath},
        node::Node,
        statement::{
            channel_declaration::ChannelDeclaration,
            expression::{
                binary_expression::BinaryExpression,
                list_expression::RangeListExpression,
                primary_expression::PrimaryExpression,
                tuple_expression::TupleExpression,
                unary_expression::UnaryExpression,
                BracketContent, BracketExpression, CallChainExpression, CallChainSegment,
                Expression, SideEffectExpression,
            },
            fn_call::FnCall,
            run_call::RunCall,
            send::SendStatement,
        },
        token::{
            args_list::ArgsList,
            binary_operator::BinaryOperator,
            datatype::DataType,
            identifier::Identifier,
            literal::Literal,
            unary_operator::UnaryOperator,
        },
    },
    error::{AlthreadError, ErrorType, Pos},
    parser::syntax::{SyntaxBlock, SyntaxBlockDetail, SyntaxBlockKind, SyntaxProgram, SyntaxSnippet},
};
use ordered_float::OrderedFloat;

pub(super) fn parse_program<'src>(
    source: &'src str,
    file_path: &str,
) -> Result<SyntaxProgram, AlthreadError> {
    let parser = custom::<_, _, _, extra::Err<Simple<'src, char>>>(move |inp| {
        let syntax = scan_program(source, file_path)?;
        while inp.next().is_some() {}
        Ok(syntax)
    })
    .then_ignore(end());

    parser
        .parse(source)
        .into_result()
        .map_err(|errs| map_chumsky_errors(source, file_path, errs))
}

pub(crate) fn parse_datatype(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<DataType>, AlthreadError> {
    let mut index = 0;
    let datatype = scan_datatype(snippet.text.as_str(), source, file_path, snippet.pos.start, &mut index)?;
    skip_inline_ws_and_comments(snippet.text.as_str(), &mut index)?;
    if index != snippet.text.len() {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start + index,
            snippet.pos.start + index + 1,
            "unexpected trailing input in datatype",
        ));
    }
    Ok(datatype)
}

pub(crate) fn parse_args_list(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<ArgsList>, AlthreadError> {
    let text = snippet.text.as_str();
    let mut index = 0;
    skip_inline_ws_and_comments(text, &mut index)?;
    let start = index;
    expect_char(text, source, file_path, snippet.pos.start, &mut index, '(')?;

    let mut identifiers = Vec::new();
    let mut datatypes = Vec::new();

    loop {
        skip_inline_ws_and_comments(text, &mut index)?;
        if index >= text.len() {
            return Err(scan_error(
                source,
                file_path,
                snippet.pos.start + start,
                snippet.pos.start + text.len(),
                "unterminated argument list",
            ));
        }

        if text.as_bytes()[index] == b')' {
            index += 1;
            break;
        }

        identifiers.push(scan_identifier(text, source, file_path, snippet.pos.start, &mut index)?);
        skip_inline_ws_and_comments(text, &mut index)?;
        expect_char(text, source, file_path, snippet.pos.start, &mut index, ':')?;
        datatypes.push(scan_datatype(
            text,
            source,
            file_path,
            snippet.pos.start,
            &mut index,
        )?);
        skip_inline_ws_and_comments(text, &mut index)?;

        match text.as_bytes().get(index).copied() {
            Some(b',') => index += 1,
            Some(b')') => {
                index += 1;
                break;
            }
            Some(_) => {
                return Err(scan_error(
                    source,
                    file_path,
                    snippet.pos.start + index,
                    snippet.pos.start + index + 1,
                    "expected ',' or ')' in argument list",
                ));
            }
            None => {
                return Err(scan_error(
                    source,
                    file_path,
                    snippet.pos.start + start,
                    snippet.pos.start + text.len(),
                    "unterminated argument list",
                ));
            }
        }
    }

    skip_inline_ws_and_comments(text, &mut index)?;
    if index != text.len() {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start + index,
            snippet.pos.start + index + 1,
            "unexpected trailing input in argument list",
        ));
    }

    Ok(Node {
        pos: Pos::from_offsets(source, file_path, snippet.pos.start + start, snippet.pos.start + index),
        value: ArgsList {
            identifiers,
            datatypes,
        },
    })
}

pub(crate) fn parse_object_identifier(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<crate::ast::token::object_identifier::ObjectIdentifier>, AlthreadError> {
    let mut index = 0;
    let ident = scan_object_identifier_node(
        snippet.text.as_str(),
        source,
        file_path,
        snippet.pos.start,
        &mut index,
    )?;
    skip_inline_ws_and_comments(snippet.text.as_str(), &mut index)?;
    if index != snippet.text.len() {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start + index,
            snippet.pos.start + index + 1,
            "unexpected trailing input in object identifier",
        ));
    }
    Ok(ident)
}

pub(crate) fn parse_fn_call(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<FnCall>, AlthreadError> {
    let text = snippet.text.as_str();
    let mut index = 0;
    let fn_name = scan_object_identifier_node(text, source, file_path, snippet.pos.start, &mut index)?;
    let (tuple_snippet, tuple_end) =
        sub_snippet_from(text, source, file_path, snippet.pos.start, index)?;
    let values = Box::new(parse_tuple_expression_node(source, &tuple_snippet, file_path)?);
    let end = tuple_snippet.pos.end;
    index = tuple_end;
    skip_inline_ws_and_comments(text, &mut index)?;
    if index != text.len() {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start + index,
            snippet.pos.end,
            "unexpected trailing input in function call",
        ));
    }
    Ok(Node {
        pos: Pos::from_offsets(source, file_path, snippet.pos.start, end),
        value: FnCall { fn_name, values },
    })
}

pub(crate) fn parse_run_call(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<RunCall>, AlthreadError> {
    let text = snippet.text.as_str();
    let mut index = 0;
    skip_inline_ws_and_comments(text, &mut index)?;
    if !consume_word(text, &mut index, "run") {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start,
            snippet.pos.end,
            "expected 'run'",
        ));
    }
    let identifier = scan_object_identifier_node(text, source, file_path, snippet.pos.start, &mut index)?;
    let (tuple_snippet, tuple_end) =
        sub_snippet_from(text, source, file_path, snippet.pos.start, index)?;
    let args = parse_tuple_expression_node(source, &tuple_snippet, file_path)?;
    index = tuple_end;
    skip_inline_ws_and_comments(text, &mut index)?;
    if index != text.len() {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start + index,
            snippet.pos.end,
            "unexpected trailing input in run call",
        ));
    }
    Ok(Node {
        pos: Pos::from_offsets(source, file_path, snippet.pos.start, snippet.pos.end),
        value: RunCall { identifier, args },
    })
}

pub(crate) fn parse_send_call(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<SendStatement>, AlthreadError> {
    let text = snippet.text.as_str();
    let mut index = 0;
    skip_inline_ws_and_comments(text, &mut index)?;
    if !consume_word(text, &mut index, "send") {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start,
            snippet.pos.end,
            "expected 'send'",
        ));
    }
    let channel = scan_object_identifier_node(text, source, file_path, snippet.pos.start, &mut index)?;
    skip_inline_ws_and_comments(text, &mut index)?;
    let is_broadcast = if text[index..].starts_with(".*") {
        index += 2;
        true
    } else {
        false
    };
    let (tuple_snippet, tuple_end) =
        sub_snippet_from(text, source, file_path, snippet.pos.start, index)?;
    let values = parse_tuple_expression_node(source, &tuple_snippet, file_path)?;
    index = tuple_end;
    skip_inline_ws_and_comments(text, &mut index)?;
    if index != text.len() {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start + index,
            snippet.pos.end,
            "unexpected trailing input in send call",
        ));
    }
    Ok(Node {
        pos: Pos::from_offsets(source, file_path, snippet.pos.start, snippet.pos.end),
        value: SendStatement {
            channel: channel
                .value
                .parts
                .iter()
                .map(|p| p.value.value.as_str())
                .collect::<Vec<_>>()
                .join("."),
            is_broadcast,
            values,
        },
    })
}

pub(crate) fn parse_channel_declaration(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<ChannelDeclaration>, AlthreadError> {
    let text = snippet.text.as_str();
    let mut index = 0;
    skip_inline_ws_and_comments(text, &mut index)?;
    if !consume_word(text, &mut index, "channel") {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start,
            snippet.pos.end,
            "expected 'channel'",
        ));
    }
    let left = scan_object_identifier_node(text, source, file_path, snippet.pos.start, &mut index)?;
    skip_inline_ws_and_comments(text, &mut index)?;
    if text.as_bytes().get(index) == Some(&b'<') {
        index += 1;
    }
    expect_char(text, source, file_path, snippet.pos.start, &mut index, '(')?;
    let mut datatypes = Vec::new();
    loop {
        skip_inline_ws_and_comments(text, &mut index)?;
        if text.as_bytes().get(index) == Some(&b')') {
            index += 1;
            break;
        }
        let datatype = scan_datatype(text, source, file_path, snippet.pos.start, &mut index)?;
        datatypes.push(datatype.value);
        skip_inline_ws_and_comments(text, &mut index)?;
        match text.as_bytes().get(index).copied() {
            Some(b',') => index += 1,
            Some(b')') => {
                index += 1;
                break;
            }
            _ => {
                return Err(scan_error(
                    source,
                    file_path,
                    snippet.pos.start + index,
                    snippet.pos.end,
                    "expected ',' or ')' in channel type list",
                ));
            }
        }
    }
    skip_inline_ws_and_comments(text, &mut index)?;
    if text.as_bytes().get(index) == Some(&b'>') {
        index += 1;
    }
    let right = scan_object_identifier_node(text, source, file_path, snippet.pos.start, &mut index)?;
    skip_inline_ws_and_comments(text, &mut index)?;
    if index != text.len() {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start + index,
            snippet.pos.end,
            "unexpected trailing input in channel declaration",
        ));
    }
    let (ch_left_prog, ch_left_name) = split_channel_endpoint(&left);
    let (ch_right_prog, ch_right_name) = split_channel_endpoint(&right);
    Ok(Node {
        pos: Pos::from_offsets(source, file_path, snippet.pos.start, snippet.pos.end),
        value: ChannelDeclaration {
            ch_left_prog,
            ch_left_name,
            ch_right_prog,
            ch_right_name,
            datatypes,
        },
    })
}

pub(crate) fn parse_import_block(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<ImportBlock>, AlthreadError> {
    let text = snippet.text.as_str();
    let mut index = 0;
    skip_inline_ws_and_comments(text, &mut index)?;
    if !consume_word(text, &mut index, "import") {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start,
            snippet.pos.end,
            "expected 'import'",
        ));
    }
    expect_char(text, source, file_path, snippet.pos.start, &mut index, '[')?;
    let mut imports = Vec::new();
    loop {
        skip_inline_ws_and_comments(text, &mut index)?;
        if text.as_bytes().get(index) == Some(&b']') {
            index += 1;
            break;
        }
        let item_start = index;
        let mut segments = Vec::new();
        loop {
            skip_inline_ws_and_comments(text, &mut index)?;
            let seg_start = index;
            let seg_end = consume_import_segment(text, index).ok_or_else(|| {
                scan_error(
                    source,
                    file_path,
                    snippet.pos.start + index,
                    snippet.pos.end,
                    "expected import path segment",
                )
            })?;
            segments.push(text[seg_start..seg_end].to_string());
            index = seg_end;
            if text.as_bytes().get(index) != Some(&b'/') {
                break;
            }
            index += 1;
        }
        skip_inline_ws_and_comments(text, &mut index)?;
        let alias = if consume_word(text, &mut index, "as") {
            Some(scan_identifier(text, source, file_path, snippet.pos.start, &mut index)?)
        } else {
            None
        };
        let item_end = index;
        imports.push(Node {
            pos: Pos::from_offsets(
                source,
                file_path,
                snippet.pos.start + item_start,
                snippet.pos.start + item_end,
            ),
            value: ImportItem {
                path: ImportPath { segments },
                alias,
            },
        });
        skip_inline_ws_and_comments(text, &mut index)?;
        match text.as_bytes().get(index).copied() {
            Some(b',') => index += 1,
            Some(b']') => {
                index += 1;
                break;
            }
            _ => {
                return Err(scan_error(
                    source,
                    file_path,
                    snippet.pos.start + index,
                    snippet.pos.end,
                    "expected ',' or ']' in import block",
                ));
            }
        }
    }
    skip_inline_ws_and_comments(text, &mut index)?;
    if index != text.len() {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start + index,
            snippet.pos.end,
            "unexpected trailing input in import block",
        ));
    }
    ImportBlock::validate_import_names(&imports)?;
    Ok(Node {
        pos: snippet.pos.clone(),
        value: ImportBlock { imports },
    })
}

pub(crate) fn parse_expression(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<Expression>, AlthreadError> {
    let mut parser = ExprParser::new(source, file_path, snippet);
    parser.parse_expression(0)
}

pub(crate) fn parse_side_effect_expression(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<SideEffectExpression>, AlthreadError> {
    let text = snippet.text.as_str();
    let mut index = 0;
    skip_inline_ws_and_comments(text, &mut index)?;
    if starts_with_word_local(text, index, "run") {
        return Ok(Node {
            pos: snippet.pos.clone(),
            value: SideEffectExpression::RunCall(parse_run_call(source, snippet, file_path)?),
        });
    }
    if text.as_bytes().get(index) == Some(&b'[') {
        return parse_bracket_expression(source, snippet, file_path);
    }
    if looks_like_direct_fn_call(text, source, file_path, snippet.pos.start, index)? {
        return Ok(Node {
            pos: snippet.pos.clone(),
            value: SideEffectExpression::FnCall(parse_fn_call(source, snippet, file_path)?),
        });
    }
    Ok(Node {
        pos: snippet.pos.clone(),
        value: SideEffectExpression::Expression(parse_expression(source, snippet, file_path)?),
    })
}

pub(crate) fn parse_list_expression(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<Expression>, AlthreadError> {
    if let Some((range_start, range_end)) = split_top_level_range(snippet.text.as_str())? {
        let (left_start, left_end) = trim_segment_bounds(snippet.text.as_str(), 0, range_start);
        let right_start = snippet.text[range_end..]
            .chars()
            .take_while(|ch| ch.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>()
            + range_end;
        let right_end = snippet.text.len();
        let left_snippet = SyntaxSnippet::new(
            Pos::from_offsets(
                source,
                file_path,
                snippet.pos.start + left_start,
                snippet.pos.start + left_end,
            ),
            snippet.text[left_start..left_end].to_string(),
        );
        let right_snippet = SyntaxSnippet::new(
            Pos::from_offsets(
                source,
                file_path,
                snippet.pos.start + right_start,
                snippet.pos.start + right_end,
            ),
            snippet.text[right_start..right_end].to_string(),
        );
        let left = parse_expression(source, &left_snippet, file_path)?;
        let right = parse_expression(source, &right_snippet, file_path)?;
        Ok(Node {
            pos: snippet.pos.clone(),
            value: Expression::Range(Node {
                pos: snippet.pos.clone(),
                value: RangeListExpression {
                    expression_start: Box::new(left),
                    expression_end: Box::new(right),
                },
            }),
        })
    } else {
        parse_expression(source, snippet, file_path)
    }
}

pub(crate) fn parse_statement_block(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<(Pos, Vec<SyntaxSnippet>), AlthreadError> {
    let (body_end, pos, snippets) = scan_statement_block(source, file_path, snippet.pos.start)
        .map_err(|err| map_chumsky_errors(source, file_path, vec![err]))?;
    if body_end != snippet.pos.end {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start,
            snippet.pos.end,
            "unexpected trailing input after code block",
        ));
    }
    Ok((pos, snippets))
}

fn scan_program<'src>(
    source: &'src str,
    file_path: &str,
) -> Result<SyntaxProgram, Simple<'src, char>> {
    let mut index = 0;
    let mut blocks = Vec::new();

    loop {
        skip_ws_and_comments(source, &mut index)?;
        if index >= source.len() {
            break;
        }

        let block_start = index;
        let kind = if starts_with_keyword(source, index, "@private") {
            index += "@private".len();
            skip_ws_and_comments(source, &mut index)?;
            parse_block_kind(source, index)?
        } else {
            parse_block_kind(source, index)?
        };

        let block = match kind {
            SyntaxBlockKind::Import => {
                consume_keyword(source, &mut index, "import")?;
                skip_ws_and_comments(source, &mut index)?;
                index = consume_balanced(source, index, b'[', b']')?;
                SyntaxBlock::new(
                    kind,
                    Pos::from_offsets(source, file_path, block_start, index),
                    source[block_start..index].to_string(),
                )
            }
            SyntaxBlockKind::Main => {
                consume_keyword(source, &mut index, "main")?;
                skip_ws_and_comments(source, &mut index)?;
                let (body_end, body_pos, body) = scan_statement_block(source, file_path, index)?;
                index = body_end;
                SyntaxBlock {
                    kind,
                    pos: Pos::from_offsets(source, file_path, block_start, index),
                    text: source[block_start..index].to_string(),
                    detail: SyntaxBlockDetail::Main { body_pos, body },
                }
            }
            SyntaxBlockKind::Global => {
                consume_keyword(source, &mut index, "shared")?;
                skip_ws_and_comments(source, &mut index)?;
                let (body_end, body_pos, body) = scan_statement_block(source, file_path, index)?;
                index = body_end;
                SyntaxBlock {
                    kind,
                    pos: Pos::from_offsets(source, file_path, block_start, index),
                    text: source[block_start..index].to_string(),
                    detail: SyntaxBlockDetail::Global { body_pos, body },
                }
            }
            SyntaxBlockKind::Always | SyntaxBlockKind::Never => {
                let keyword = if kind == SyntaxBlockKind::Always {
                    "always"
                } else {
                    "never"
                };
                consume_keyword(source, &mut index, keyword)?;
                skip_ws_and_comments(source, &mut index)?;
                let (body_end, body_pos, body) = scan_expression_block(source, file_path, index)?;
                index = body_end;
                SyntaxBlock {
                    kind,
                    pos: Pos::from_offsets(source, file_path, block_start, index),
                    text: source[block_start..index].to_string(),
                    detail: SyntaxBlockDetail::Condition { body_pos, body },
                }
            }
            SyntaxBlockKind::Check => {
                consume_keyword(source, &mut index, "check")?;
                skip_ws_and_comments(source, &mut index)?;
                let (body_end, body_pos, formulas) = scan_ltl_block(source, file_path, index)?;
                index = body_end;
                SyntaxBlock {
                    kind,
                    pos: Pos::from_offsets(source, file_path, block_start, index),
                    text: source[block_start..index].to_string(),
                    detail: SyntaxBlockDetail::Check { body_pos, formulas },
                }
            }
            SyntaxBlockKind::Program => {
                let is_private = if starts_with_keyword(source, index, "@private") {
                    consume_keyword(source, &mut index, "@private")?;
                    skip_ws_and_comments(source, &mut index)?;
                    true
                } else {
                    false
                };
                consume_keyword(source, &mut index, "program")?;
                skip_ws_and_comments(source, &mut index)?;
                let name = parse_identifier_snippet(source, file_path, &mut index)?;
                skip_ws_and_comments(source, &mut index)?;
                let args = parse_balanced_snippet(source, file_path, &mut index, b'(', b')')?;
                skip_ws_and_comments(source, &mut index)?;
                let (body_end, body_pos, body) = scan_statement_block(source, file_path, index)?;
                index = body_end;
                SyntaxBlock {
                    kind,
                    pos: Pos::from_offsets(source, file_path, block_start, index),
                    text: source[block_start..index].to_string(),
                    detail: SyntaxBlockDetail::Program {
                        is_private,
                        name,
                        args,
                        body_pos,
                        body,
                    },
                }
            }
            SyntaxBlockKind::Function => {
                let is_private = if starts_with_keyword(source, index, "@private") {
                    consume_keyword(source, &mut index, "@private")?;
                    skip_ws_and_comments(source, &mut index)?;
                    true
                } else {
                    false
                };
                consume_keyword(source, &mut index, "fn")?;
                skip_ws_and_comments(source, &mut index)?;
                let name = parse_identifier_snippet(source, file_path, &mut index)?;
                skip_ws_and_comments(source, &mut index)?;
                let args = parse_balanced_snippet(source, file_path, &mut index, b'(', b')')?;
                skip_ws_and_comments(source, &mut index)?;
                if !source[index..].starts_with("->") {
                    return Err(simple_error(
                        index,
                        source,
                        "expected '->' after function arguments",
                    ));
                }
                index += 2;
                skip_ws_and_comments(source, &mut index)?;
                let return_start = index;
                index = consume_until_block(source, index)?;
                let return_type = SyntaxSnippet::new(
                    Pos::from_offsets(source, file_path, return_start, index),
                    source[return_start..index].trim().to_string(),
                );
                let (body_end, body_pos, body) = scan_statement_block(source, file_path, index)?;
                index = body_end;
                SyntaxBlock {
                    kind,
                    pos: Pos::from_offsets(source, file_path, block_start, index),
                    text: source[block_start..index].to_string(),
                    detail: SyntaxBlockDetail::Function {
                        is_private,
                        name,
                        args,
                        return_type,
                        body_pos,
                        body,
                    },
                }
            }
        };

        blocks.push(block);
    }

    Ok(SyntaxProgram { blocks })
}

fn scan_datatype(
    text: &str,
    source: &str,
    file_path: &str,
    base_offset: usize,
    index: &mut usize,
) -> Result<Node<DataType>, AlthreadError> {
    skip_inline_ws_and_comments(text, index)?;
    let start = *index;

    let value = if consume_word(text, index, "bool") {
        DataType::Boolean
    } else if consume_word(text, index, "int") {
        DataType::Integer
    } else if consume_word(text, index, "float") {
        DataType::Float
    } else if consume_word(text, index, "string") {
        DataType::String
    } else if consume_word(text, index, "void") {
        DataType::Void
    } else if consume_word(text, index, "proc") {
        skip_inline_ws_and_comments(text, index)?;
        expect_char(text, source, file_path, base_offset, index, '(')?;
        let name = scan_object_identifier_text(text, source, file_path, base_offset, index)?;
        skip_inline_ws_and_comments(text, index)?;
        expect_char(text, source, file_path, base_offset, index, ')')?;
        DataType::Process(name)
    } else if consume_word(text, index, "list") {
        skip_inline_ws_and_comments(text, index)?;
        expect_char(text, source, file_path, base_offset, index, '(')?;
        let inner = scan_datatype(text, source, file_path, base_offset, index)?;
        skip_inline_ws_and_comments(text, index)?;
        expect_char(text, source, file_path, base_offset, index, ')')?;
        DataType::List(Box::new(inner.value))
    } else if consume_word(text, index, "tuple") {
        skip_inline_ws_and_comments(text, index)?;
        expect_char(text, source, file_path, base_offset, index, '(')?;
        let mut items = Vec::new();
        loop {
            items.push(scan_datatype(text, source, file_path, base_offset, index)?.value);
            skip_inline_ws_and_comments(text, index)?;
            match text.as_bytes().get(*index).copied() {
                Some(b',') => *index += 1,
                Some(b')') => {
                    *index += 1;
                    break;
                }
                Some(_) => {
                    return Err(scan_error(
                        source,
                        file_path,
                        base_offset + *index,
                        base_offset + *index + 1,
                        "expected ',' or ')' in tuple datatype",
                    ));
                }
                None => {
                    return Err(scan_error(
                        source,
                        file_path,
                        base_offset + start,
                        base_offset + text.len(),
                        "unterminated tuple datatype",
                    ));
                }
            }
        }
        DataType::Tuple(items)
    } else {
        return Err(scan_error(
            source,
            file_path,
            base_offset + start,
            base_offset + (*index + 1).min(text.len()),
            "expected a datatype",
        ));
    };

    Ok(Node {
        pos: Pos::from_offsets(source, file_path, base_offset + start, base_offset + *index),
        value,
    })
}

struct ExprParser<'a> {
    source: &'a str,
    file_path: &'a str,
    snippet: &'a SyntaxSnippet,
    text: &'a str,
    index: usize,
}

impl<'a> ExprParser<'a> {
    fn new(source: &'a str, file_path: &'a str, snippet: &'a SyntaxSnippet) -> Self {
        Self {
            source,
            file_path,
            snippet,
            text: snippet.text.as_str(),
            index: 0,
        }
    }

    fn parse_expression(&mut self, min_prec: u8) -> Result<Node<Expression>, AlthreadError> {
        skip_inline_ws_and_comments(self.text, &mut self.index)?;
        let expr_start = self.index;
        let mut left = self.parse_prefix()?;

        loop {
            skip_inline_ws_and_comments(self.text, &mut self.index)?;
            let Some((operator, op_start, op_len, precedence)) = self.peek_binary_operator()? else {
                break;
            };
            if precedence < min_prec {
                break;
            }
            self.index += op_len;
            let right = self.parse_expression(precedence + 1)?;
            let left_operand = normalize_binary_operand(left);
            let right_operand = normalize_binary_operand(right);
            let op_pos = Pos::from_offsets(
                self.source,
                self.file_path,
                self.snippet.pos.start + op_start,
                self.snippet.pos.start + op_start + op_len,
            );
            let binary = Node {
                pos: Pos::from_offsets(
                    self.source,
                    self.file_path,
                    left_operand.pos.start,
                    right_operand.pos.end,
                ),
                value: BinaryExpression {
                    left: Box::new(left_operand),
                    operator: Node {
                        pos: op_pos,
                        value: operator,
                    },
                    right: Box::new(right_operand),
                },
            };
            left = Node {
                pos: Pos::from_offsets(
                    self.source,
                    self.file_path,
                    self.snippet.pos.start + expr_start,
                    binary.pos.end,
                ),
                value: Expression::Binary(binary),
            };
        }

        skip_inline_ws_and_comments(self.text, &mut self.index)?;
        if self.index != self.text.len() && min_prec == 0 {
            return Err(scan_error(
                self.source,
                self.file_path,
                self.snippet.pos.start + self.index,
                self.snippet.pos.end,
                "unexpected trailing input in expression",
            ));
        }

        if min_prec == 0 {
            left.pos = Pos::from_offsets(
                self.source,
                self.file_path,
                self.snippet.pos.start + expr_start,
                self.snippet.pos.end,
            );
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Node<Expression>, AlthreadError> {
        skip_inline_ws_and_comments(self.text, &mut self.index)?;
        let start = self.index;
        if let Some((operator, len)) = self.peek_unary_operator() {
            self.index += len;
            let operand = self.parse_prefix()?;
            let operand_end = operand.pos.end;
            let op_pos = Pos::from_offsets(
                self.source,
                self.file_path,
                self.snippet.pos.start + start,
                self.snippet.pos.start + start + len,
            );
            let unary = Node {
                pos: Pos::from_offsets(
                    self.source,
                    self.file_path,
                    self.snippet.pos.start + start,
                    operand_end,
                ),
                value: UnaryExpression {
                    operator: Node {
                        pos: op_pos,
                        value: operator,
                    },
                    operand: Box::new(operand),
                },
            };
            return Ok(Node {
                pos: unary.pos.clone(),
                value: Expression::Unary(unary),
            });
        }

        self.parse_postfix_primary()
    }

    fn parse_postfix_primary(&mut self) -> Result<Node<Expression>, AlthreadError> {
        let start = self.index;
        let mut base = self.parse_primary()?;
        let mut segments = Vec::new();

        loop {
            let saved = self.index;
            skip_inline_ws_and_comments(self.text, &mut self.index)?;
            if self.text.as_bytes().get(self.index) != Some(&b'.') {
                self.index = saved;
                break;
            }
            let dot_index = self.index;
            self.index += 1;
            skip_inline_ws_and_comments(self.text, &mut self.index)?;
            if starts_with_word_local(self.text, self.index, "reaches") {
                self.index += "reaches".len();
                let label_start = self.index;
                let (tuple_snippet, tuple_end) = sub_snippet_from(
                    self.text,
                    self.source,
                    self.file_path,
                    self.snippet.pos.start,
                    self.index,
                )?;
                self.index = tuple_end;
                let mut tuple_index = 0;
                expect_char(
                    tuple_snippet.text.as_str(),
                    self.source,
                    self.file_path,
                    tuple_snippet.pos.start,
                    &mut tuple_index,
                    '(',
                )?;
                let label = scan_identifier(
                    tuple_snippet.text.as_str(),
                    self.source,
                    self.file_path,
                    tuple_snippet.pos.start,
                    &mut tuple_index,
                )?;
                let _ = label_start;
                segments.push(CallChainSegment::Reaches { label });
            } else {
                let name = scan_identifier(
                    self.text,
                    self.source,
                    self.file_path,
                    self.snippet.pos.start,
                    &mut self.index,
                )?;
                let (tuple_snippet, tuple_end) = sub_snippet_from(
                    self.text,
                    self.source,
                    self.file_path,
                    self.snippet.pos.start,
                    self.index,
                )?;
                let args = parse_tuple_expression_node(self.source, &tuple_snippet, self.file_path)?;
                self.index = tuple_end;
                segments.push(CallChainSegment::Call { name, args });
            }

            if dot_index < start {
                break;
            }
        }

        if segments.is_empty() {
            return Ok(base);
        }

        if let Expression::FnCall(call_node) = &base.value {
            let parts = call_node.value.fn_name.value.parts.clone();
            if parts.len() > 1 {
                let base_parts = parts[..parts.len() - 1].to_vec();
                let base_ident = Node {
                    pos: call_node.value.fn_name.pos.clone(),
                    value: crate::ast::token::object_identifier::ObjectIdentifier {
                        parts: base_parts,
                    },
                };
                let base_primary = Node {
                    pos: base_ident.pos.clone(),
                    value: PrimaryExpression::Identifier(base_ident),
                };
                let args = (*call_node.value.values).clone();
                segments.insert(
                    0,
                    CallChainSegment::Call {
                        name: parts.last().cloned().unwrap(),
                        args,
                    },
                );
                base = Node {
                    pos: base_primary.pos.clone(),
                    value: Expression::Primary(base_primary),
                };
            }
        }

        let end = self.snippet.pos.start + self.index;
        let pos = Pos::from_offsets(self.source, self.file_path, base.pos.start, end);
        Ok(Node {
            pos: pos.clone(),
            value: Expression::CallChain(Node {
                pos,
                value: CallChainExpression {
                    base: Box::new(base),
                    segments,
                },
            }),
        })
    }

    fn parse_primary(&mut self) -> Result<Node<Expression>, AlthreadError> {
        skip_inline_ws_and_comments(self.text, &mut self.index)?;
        let start = self.index;
        if self.text.as_bytes().get(self.index) == Some(&b'(') {
            let end = consume_balanced(self.text, self.index, b'(', b')')
                .map_err(|err| map_chumsky_errors(self.text, self.file_path, vec![err]))?;
            let inner = SyntaxSnippet::new(
                Pos::from_offsets(
                    self.source,
                    self.file_path,
                    self.snippet.pos.start + start + 1,
                    self.snippet.pos.start + end - 1,
                ),
                self.text[start + 1..end - 1].to_string(),
            );
            self.index = end;
            let expr = parse_expression(self.source, &inner, self.file_path)?;
            let primary = Node {
                pos: expr.pos.clone(),
                value: PrimaryExpression::Expression(Box::new(expr)),
            };
            return Ok(Node {
                pos: primary.pos.clone(),
                value: Expression::Primary(primary),
            });
        }
        if self.text.as_bytes().get(self.index) == Some(&b'"')
            || self
                .text
                .as_bytes()
                .get(self.index)
                .copied()
                .is_some_and(|b| b.is_ascii_digit())
            || starts_with_word_local(self.text, self.index, "true")
            || starts_with_word_local(self.text, self.index, "false")
            || starts_with_word_local(self.text, self.index, "null")
        {
            let literal = parse_literal_node(
                self.text,
                self.source,
                self.file_path,
                self.snippet.pos.start,
                &mut self.index,
            )?;
            let primary = Node {
                pos: literal.pos.clone(),
                value: PrimaryExpression::Literal(literal),
            };
            return Ok(Node {
                pos: primary.pos.clone(),
                value: Expression::Primary(primary),
            });
        }

        if starts_with_word_local(self.text, self.index, "if") {
            return self.parse_if_expression();
        }
        if starts_with_word_local(self.text, self.index, "for") {
            return self.parse_quantified_expression(true);
        }
        if starts_with_word_local(self.text, self.index, "exists") {
            return self.parse_quantified_expression(false);
        }

        let object = scan_object_identifier_node(
            self.text,
            self.source,
            self.file_path,
            self.snippet.pos.start,
            &mut self.index,
        )?;
        skip_inline_ws_and_comments(self.text, &mut self.index)?;
        if self.text.as_bytes().get(self.index) == Some(&b'(') {
            let (_, tuple_end) = sub_snippet_from(
                self.text,
                self.source,
                self.file_path,
                self.snippet.pos.start,
                self.index,
            )?;
            let fn_snippet = SyntaxSnippet::new(
                Pos::from_offsets(
                    self.source,
                    self.file_path,
                    self.snippet.pos.start + start,
                    self.snippet.pos.start + tuple_end,
                ),
                self.text[start..tuple_end].to_string(),
            );
            let call = parse_fn_call(self.source, &fn_snippet, self.file_path)?;
            self.index = tuple_end;
            return Ok(Node {
                pos: call.pos.clone(),
                value: Expression::FnCall(call),
            });
        }
        let primary = Node {
            pos: object.pos.clone(),
            value: PrimaryExpression::Identifier(object),
        };
        Ok(Node {
            pos: primary.pos.clone(),
            value: Expression::Primary(primary),
        })
    }

    fn parse_if_expression(&mut self) -> Result<Node<Expression>, AlthreadError> {
        let start = self.index;
        self.index += 2;
        let cond_start = self.index;
        let then_start = find_top_level_block_start(self.text, cond_start).ok_or_else(|| {
            scan_error(
                self.source,
                self.file_path,
                self.snippet.pos.start + cond_start,
                self.snippet.pos.end,
                "expected block in if expression",
            )
        })?;
        let cond_snippet = SyntaxSnippet::new(
            Pos::from_offsets(
                self.source,
                self.file_path,
                self.snippet.pos.start + cond_start,
                self.snippet.pos.start + then_start,
            ),
            self.text[cond_start..then_start].to_string(),
        );
        let condition = parse_expression(self.source, &cond_snippet, self.file_path)?;
        let then_end = consume_balanced_block(self.text, then_start, '{', '}').ok_or_else(|| {
            scan_error(
                self.source,
                self.file_path,
                self.snippet.pos.start + then_start,
                self.snippet.pos.end,
                "unterminated if-expression block",
            )
        })?;
        let then_snippet = SyntaxSnippet::new(
            Pos::from_offsets(
                self.source,
                self.file_path,
                self.snippet.pos.start + then_start + 1,
                self.snippet.pos.start + then_end - 1,
            ),
            self.text[then_start + 1..then_end - 1].trim().trim_end_matches(';').to_string(),
        );
        let then_expr = parse_expression(self.source, &then_snippet, self.file_path)?;
        self.index = then_end;
        skip_inline_ws_and_comments(self.text, &mut self.index)?;
        let else_expr = if starts_with_word_local(self.text, self.index, "else") {
            self.index += 4;
            skip_inline_ws_and_comments(self.text, &mut self.index)?;
            let else_start = self.index;
            let else_end = consume_balanced_block(self.text, else_start, '{', '}').ok_or_else(|| {
                scan_error(
                    self.source,
                    self.file_path,
                    self.snippet.pos.start + else_start,
                    self.snippet.pos.end,
                    "unterminated else-expression block",
                )
            })?;
            let else_snippet = SyntaxSnippet::new(
                Pos::from_offsets(
                    self.source,
                    self.file_path,
                    self.snippet.pos.start + else_start + 1,
                    self.snippet.pos.start + else_end - 1,
                ),
                self.text[else_start + 1..else_end - 1]
                    .trim()
                    .trim_end_matches(';')
                    .to_string(),
            );
            self.index = else_end;
            Some(Box::new(parse_expression(self.source, &else_snippet, self.file_path)?))
        } else {
            None
        };
        let pos = Pos::from_offsets(
            self.source,
            self.file_path,
            self.snippet.pos.start + start,
            self.snippet.pos.start + self.index,
        );
        Ok(Node {
            pos: pos.clone(),
            value: Expression::Primary(Node {
                pos,
                value: PrimaryExpression::IfExpr {
                    condition: Box::new(condition),
                    then_expr: Box::new(then_expr),
                    else_expr,
                },
            }),
        })
    }

    fn parse_quantified_expression(
        &mut self,
        is_forall: bool,
    ) -> Result<Node<Expression>, AlthreadError> {
        let start = self.index;
        self.index += if is_forall { 3 } else { 6 };
        let var = scan_identifier(
            self.text,
            self.source,
            self.file_path,
            self.snippet.pos.start,
            &mut self.index,
        )?;
        skip_inline_ws_and_comments(self.text, &mut self.index)?;
        if !consume_word(self.text, &mut self.index, "in") {
            return Err(scan_error(
                self.source,
                self.file_path,
                self.snippet.pos.start + self.index,
                self.snippet.pos.end,
                "expected 'in' in quantified expression",
            ));
        }
        let list_start = self.index;
        let body_start = find_top_level_block_start(self.text, list_start).ok_or_else(|| {
            scan_error(
                self.source,
                self.file_path,
                self.snippet.pos.start + list_start,
                self.snippet.pos.end,
                "expected body block in quantified expression",
            )
        })?;
        let list_snippet = SyntaxSnippet::new(
            Pos::from_offsets(
                self.source,
                self.file_path,
                self.snippet.pos.start + list_start,
                self.snippet.pos.start + body_start,
            ),
            self.text[list_start..body_start].to_string(),
        );
        let list = parse_expression(self.source, &list_snippet, self.file_path)?;
        let body_end = consume_balanced_block(self.text, body_start, '{', '}').ok_or_else(|| {
            scan_error(
                self.source,
                self.file_path,
                self.snippet.pos.start + body_start,
                self.snippet.pos.end,
                "unterminated quantified expression block",
            )
        })?;
        let body_snippet = SyntaxSnippet::new(
            Pos::from_offsets(
                self.source,
                self.file_path,
                self.snippet.pos.start + body_start + 1,
                self.snippet.pos.start + body_end - 1,
            ),
            self.text[body_start + 1..body_end - 1]
                .trim()
                .trim_end_matches(';')
                .to_string(),
        );
        let body = parse_expression(self.source, &body_snippet, self.file_path)?;
        self.index = body_end;
        let pos = Pos::from_offsets(
            self.source,
            self.file_path,
            self.snippet.pos.start + start,
            self.snippet.pos.start + body_end,
        );
        Ok(Node {
            pos: pos.clone(),
            value: Expression::Primary(Node {
                pos,
                value: if is_forall {
                    PrimaryExpression::ForAllExpr {
                        var,
                        list: Box::new(list),
                        body: Box::new(body),
                    }
                } else {
                    PrimaryExpression::ExistsExpr {
                        var,
                        list: Box::new(list),
                        body: Box::new(body),
                    }
                },
            }),
        })
    }

    fn peek_unary_operator(&self) -> Option<(UnaryOperator, usize)> {
        let slice = &self.text[self.index..];
        if slice.starts_with('!') {
            Some((UnaryOperator::Not, 1))
        } else if slice.starts_with('+') {
            Some((UnaryOperator::Positive, 1))
        } else if slice.starts_with('-') {
            Some((UnaryOperator::Negative, 1))
        } else {
            None
        }
    }

    fn peek_binary_operator(&self) -> Result<Option<(BinaryOperator, usize, usize, u8)>, AlthreadError> {
        let mut index = self.index;
        skip_inline_ws_and_comments(self.text, &mut index)?;
        let candidates = [
            ("||", BinaryOperator::Or, 1),
            ("&&", BinaryOperator::And, 2),
            ("|", BinaryOperator::BitOr, 3),
            ("&", BinaryOperator::BitAnd, 3),
            ("==", BinaryOperator::Equals, 4),
            ("!=", BinaryOperator::NotEquals, 4),
            ("<<", BinaryOperator::ShiftLeft, 5),
            (">>", BinaryOperator::ShiftRight, 5),
            ("<=", BinaryOperator::LessThanOrEqual, 6),
            (">=", BinaryOperator::GreaterThanOrEqual, 6),
            ("<", BinaryOperator::LessThan, 6),
            (">", BinaryOperator::GreaterThan, 6),
            ("+", BinaryOperator::Add, 7),
            ("-", BinaryOperator::Subtract, 7),
            ("*", BinaryOperator::Multiply, 8),
            ("/", BinaryOperator::Divide, 8),
            ("%", BinaryOperator::Modulo, 8),
        ];
        for (token, op, prec) in candidates {
            if self.text[index..].starts_with(token) {
                return Ok(Some((op, index, token.len(), prec)));
            }
        }
        Ok(None)
    }
}

fn normalize_binary_operand(node: Node<Expression>) -> Node<Expression> {
    let operator_pos = match &node.value {
        Expression::Binary(binary) => Some(binary.value.operator.pos.clone()),
        Expression::Unary(unary) => Some(unary.value.operator.pos.clone()),
        _ => None,
    };
    match operator_pos {
        Some(pos) => {
            let mut normalized = node;
            normalized.pos = pos;
            normalized
        }
        None => node,
    }
}

fn parse_bracket_expression(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<SideEffectExpression>, AlthreadError> {
    let text = snippet.text.as_str();
    let end = consume_balanced(text, 0, b'[', b']')
        .map_err(|err| map_chumsky_errors(text, file_path, vec![err]))?;
    let inner = &text[1..end - 1];
    let inner_start = snippet.pos.start + 1;
    let content = if let Some((range_start, range_end)) = split_top_level_range(inner)? {
        let left_snippet = SyntaxSnippet::new(
            Pos::from_offsets(source, file_path, inner_start, inner_start + range_start),
            inner[..range_start].to_string(),
        );
        let right_snippet = SyntaxSnippet::new(
            Pos::from_offsets(
                source,
                file_path,
                inner_start + range_end,
                inner_start + inner.len(),
            ),
            inner[range_end..].to_string(),
        );
        let left = parse_expression(source, &left_snippet, file_path)?;
        let right = parse_expression(source, &right_snippet, file_path)?;
        BracketContent::Range(Node {
            pos: Pos::from_offsets(source, file_path, snippet.pos.start + 1, snippet.pos.end - 1),
            value: RangeListExpression {
                expression_start: Box::new(left),
                expression_end: Box::new(right),
            },
        })
    } else {
        let mut values = Vec::new();
        for (seg_start, seg_end) in split_top_level_segments(inner, 0, inner.len())? {
            if seg_start == seg_end {
                continue;
            }
            let expr_snippet = SyntaxSnippet::new(
                Pos::from_offsets(
                    source,
                    file_path,
                    inner_start + seg_start,
                    inner_start + seg_end,
                ),
                inner[seg_start..seg_end].to_string(),
            );
            values.push(parse_side_effect_expression(source, &expr_snippet, file_path)?);
        }
        BracketContent::ListLiteral(values)
    };
    Ok(Node {
        pos: snippet.pos.clone(),
        value: SideEffectExpression::Bracket(Node {
            pos: snippet.pos.clone(),
            value: BracketExpression { content },
        }),
    })
}

fn parse_literal_node(
    text: &str,
    source: &str,
    file_path: &str,
    base_offset: usize,
    index: &mut usize,
) -> Result<Node<Literal>, AlthreadError> {
    skip_inline_ws_and_comments(text, index)?;
    let start = *index;
    let value = if text.as_bytes().get(*index) == Some(&b'"') {
        let end = consume_string(text, *index).map_err(|err| map_chumsky_errors(text, file_path, vec![err]))?;
        let raw = &text[*index + 1..end - 1];
        *index = end;
        Literal::String(raw.to_string())
    } else if starts_with_word_local(text, *index, "true") {
        *index += 4;
        Literal::Bool(true)
    } else if starts_with_word_local(text, *index, "false") {
        *index += 5;
        Literal::Bool(false)
    } else if starts_with_word_local(text, *index, "null") {
        *index += 4;
        Literal::Null
    } else {
        let end = consume_number_local(text, *index).ok_or_else(|| {
            scan_error(
                source,
                file_path,
                base_offset + *index,
                base_offset + (*index + 1).min(text.len()),
                "expected literal",
            )
        })?;
        let raw = &text[*index..end];
        *index = end;
        if raw.contains('.') {
            Literal::Float(OrderedFloat(raw.parse::<f64>().map_err(|_| {
                scan_error(source, file_path, base_offset + start, base_offset + end, "invalid float literal")
            })?))
        } else if raw.starts_with("0x") || raw.starts_with("0X") {
            Literal::Int(i64::from_str_radix(&raw[2..], 16).map_err(|_| {
                scan_error(source, file_path, base_offset + start, base_offset + end, "invalid integer literal")
            })?)
        } else if raw.starts_with("0b") || raw.starts_with("0B") {
            Literal::Int(i64::from_str_radix(&raw[2..], 2).map_err(|_| {
                scan_error(source, file_path, base_offset + start, base_offset + end, "invalid integer literal")
            })?)
        } else {
            Literal::Int(raw.parse::<i64>().map_err(|_| {
                scan_error(source, file_path, base_offset + start, base_offset + end, "invalid integer literal")
            })?)
        }
    };
    Ok(Node {
        pos: Pos::from_offsets(source, file_path, base_offset + start, base_offset + *index),
        value,
    })
}

fn looks_like_direct_fn_call(
    text: &str,
    source: &str,
    file_path: &str,
    base_offset: usize,
    start: usize,
) -> Result<bool, AlthreadError> {
    let mut probe = start;
    skip_inline_ws_and_comments(text, &mut probe)?;
    let Some(first) = text.as_bytes().get(probe).copied() else {
        return Ok(false);
    };
    if first != b'$' && !first.is_ascii_alphabetic() {
        return Ok(false);
    }
    let mut index = start;
    let _ = scan_object_identifier_node(text, source, file_path, base_offset, &mut index)?;
    skip_inline_ws_and_comments(text, &mut index)?;
    Ok(text.as_bytes().get(index) == Some(&b'('))
}

fn scan_identifier(
    text: &str,
    source: &str,
    file_path: &str,
    base_offset: usize,
    index: &mut usize,
) -> Result<Node<Identifier>, AlthreadError> {
    skip_inline_ws_and_comments(text, index)?;
    let start = *index;
    *index = consume_identifier_local(text, *index).ok_or_else(|| {
        scan_error(
            source,
            file_path,
            base_offset + start,
            base_offset + (start + 1).min(text.len()),
            "expected identifier",
        )
    })?;

    Ok(Node {
        pos: Pos::from_offsets(source, file_path, base_offset + start, base_offset + *index),
        value: Identifier {
            value: text[start..*index].to_string(),
        },
    })
}

fn scan_object_identifier_node(
    text: &str,
    source: &str,
    file_path: &str,
    base_offset: usize,
    index: &mut usize,
) -> Result<Node<crate::ast::token::object_identifier::ObjectIdentifier>, AlthreadError> {
    skip_inline_ws_and_comments(text, index)?;
    let start = *index;
    let mut parts = Vec::new();

    loop {
        skip_inline_ws_and_comments(text, index)?;
        let part_start = *index;
        let value = if text.as_bytes().get(*index) == Some(&b'$') {
            *index += 1;
            "$".to_string()
        } else {
            let end = consume_identifier_local(text, *index).ok_or_else(|| {
                scan_error(
                    source,
                    file_path,
                    base_offset + *index,
                    base_offset + (*index + 1).min(text.len()),
                    "expected identifier",
                )
            })?;
            let value = text[*index..end].to_string();
            *index = end;
            value
        };
        parts.push(Node {
            pos: Pos::from_offsets(source, file_path, base_offset + part_start, base_offset + *index),
            value: Identifier { value },
        });
        skip_inline_ws_and_comments(text, index)?;
        if text.as_bytes().get(*index) != Some(&b'.') {
            break;
        }
        let next_index = *index + 1;
        if text[next_index..].starts_with("reaches") {
            let reaches_end = next_index + "reaches".len();
            let mut probe = reaches_end;
            while probe < text.len() && text.as_bytes()[probe].is_ascii_whitespace() {
                probe += 1;
            }
            if text.as_bytes().get(probe) == Some(&b'(') {
                break;
            }
        }
        match text.as_bytes().get(next_index).copied() {
            Some(b'$') => {}
            Some(next) if next.is_ascii_alphabetic() => {}
            _ => break,
        }
        *index = next_index;
    }

    Ok(Node {
        pos: Pos::from_offsets(source, file_path, base_offset + start, base_offset + *index),
        value: crate::ast::token::object_identifier::ObjectIdentifier { parts },
    })
}

fn scan_object_identifier_text(
    text: &str,
    source: &str,
    file_path: &str,
    base_offset: usize,
    index: &mut usize,
) -> Result<String, AlthreadError> {
    let first = scan_identifier(text, source, file_path, base_offset, index)?;
    let mut parts = vec![first.value.value];

    loop {
        skip_inline_ws_and_comments(text, index)?;
        if text.as_bytes().get(*index) != Some(&b'.') {
            break;
        }
        *index += 1;
        parts.push(scan_identifier(text, source, file_path, base_offset, index)?.value.value);
    }

    Ok(parts.join("."))
}

fn parse_tuple_expression_node(
    source: &str,
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Node<Expression>, AlthreadError> {
    let text = snippet.text.as_str();
    let mut index = 0;
    let start = index;
    expect_char(text, source, file_path, snippet.pos.start, &mut index, '(')?;
    let inner_start = index;
    let end = consume_balanced(text, inner_start - 1, b'(', b')').map_err(|err| {
        map_chumsky_errors(text, file_path, vec![err])
    })?;
    let close_index = end - 1;
    let mut values = Vec::new();
    for (seg_start, seg_end) in split_top_level_segments(text, inner_start, close_index)? {
        if seg_start == seg_end {
            continue;
        }
        let expr_snippet = SyntaxSnippet::new(
            Pos::from_offsets(source, file_path, snippet.pos.start + seg_start, snippet.pos.start + seg_end),
            text[seg_start..seg_end].to_string(),
        );
        values.push(parse_expression(source, &expr_snippet, file_path)?);
    }
    index = end;
    skip_inline_ws_and_comments(text, &mut index)?;
    if index != text.len() {
        return Err(scan_error(
            source,
            file_path,
            snippet.pos.start + index,
            snippet.pos.end,
            "unexpected trailing input in tuple expression",
        ));
    }
    let pos = Pos::from_offsets(source, file_path, snippet.pos.start + start, snippet.pos.start + close_index + 1);
    Ok(Node {
        pos: pos.clone(),
        value: Expression::Tuple(Node {
            pos,
            value: TupleExpression { values },
        }),
    })
}

fn split_top_level_segments(
    text: &str,
    start: usize,
    end: usize,
) -> Result<Vec<(usize, usize)>, AlthreadError> {
    let mut segments = Vec::new();
    let mut seg_start = start;
    let mut index = start;
    let mut paren = 0usize;
    let mut brace = 0usize;
    let mut bracket = 0usize;
    while index < end {
        if text[index..].starts_with("//") {
            while index < end && text.as_bytes()[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if text[index..].starts_with("/*") {
            index += 2;
            while index + 1 < end && &text[index..index + 2] != "*/" {
                index += 1;
            }
            index = (index + 2).min(end);
            continue;
        }
        match text.as_bytes()[index] {
            b'"' => {
                index += 1;
                while index < end && text.as_bytes()[index] != b'"' {
                    index += 1;
                }
                index = (index + 1).min(end);
            }
            b'(' => {
                paren += 1;
                index += 1;
            }
            b')' => {
                paren = paren.saturating_sub(1);
                index += 1;
            }
            b'{' => {
                brace += 1;
                index += 1;
            }
            b'}' => {
                brace = brace.saturating_sub(1);
                index += 1;
            }
            b'[' => {
                bracket += 1;
                index += 1;
            }
            b']' => {
                bracket = bracket.saturating_sub(1);
                index += 1;
            }
            b',' if paren == 0 && brace == 0 && bracket == 0 => {
                segments.push(trim_segment_bounds(text, seg_start, index));
                index += 1;
                seg_start = index;
            }
            _ => index += 1,
        }
    }
    if seg_start <= end {
        segments.push(trim_segment_bounds(text, seg_start, end));
    }
    Ok(segments)
}

fn trim_segment_bounds(text: &str, mut start: usize, mut end: usize) -> (usize, usize) {
    while start < end && text.as_bytes()[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && text.as_bytes()[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    (start, end)
}

fn sub_snippet_from(
    text: &str,
    source: &str,
    file_path: &str,
    base_offset: usize,
    index: usize,
) -> Result<(SyntaxSnippet, usize), AlthreadError> {
    let mut start = index;
    skip_inline_ws_and_comments(text, &mut start)?;
    let end = consume_balanced(text, start, b'(', b')')
        .map_err(|err| map_chumsky_errors(text, file_path, vec![err]))?;
    Ok((
        SyntaxSnippet::new(
            Pos::from_offsets(source, file_path, base_offset + start, base_offset + end),
            text[start..end].to_string(),
        ),
        end,
    ))
}

fn split_channel_endpoint(
    endpoint: &Node<crate::ast::token::object_identifier::ObjectIdentifier>,
) -> (String, String) {
    let mut parts = endpoint
        .value
        .parts
        .iter()
        .map(|part| part.value.value.clone())
        .collect::<Vec<_>>();
    let prog = parts.first().cloned().unwrap_or_default();
    let name = if parts.len() > 1 {
        parts.drain(1..).collect::<Vec<_>>().join(".")
    } else {
        String::new()
    };
    (prog, name)
}

fn scan_ltl_block<'src>(
    source: &'src str,
    file_path: &str,
    block_start: usize,
) -> Result<(usize, Pos, Vec<SyntaxSnippet>), Simple<'src, char>> {
    if block_start >= source.len() || source.as_bytes()[block_start] != b'{' {
        return Err(simple_error(block_start, source, "expected '{'"));
    }

    let mut index = block_start + 1;
    let mut snippets = Vec::new();
    loop {
        skip_ws_and_comments(source, &mut index)?;
        if index >= source.len() {
            return Err(simple_error(
                source.len().saturating_sub(1),
                source,
                "unterminated block",
            ));
        }
        if source.as_bytes()[index] == b'}' {
            return Ok((
                index + 1,
                Pos::from_offsets(source, file_path, block_start, index + 1),
                snippets,
            ));
        }

        let expr_start = index;
        index = consume_semicolon_terminated(source, expr_start)?;
        let expr_end = index.saturating_sub(1);
        snippets.push(SyntaxSnippet::new(
            Pos::from_offsets(source, file_path, expr_start, expr_end),
            source[expr_start..expr_end].trim().to_string(),
        ));
    }
}

fn scan_statement_block<'src>(
    source: &'src str,
    file_path: &str,
    block_start: usize,
) -> Result<(usize, Pos, Vec<SyntaxSnippet>), Simple<'src, char>> {
    if block_start >= source.len() || source.as_bytes()[block_start] != b'{' {
        return Err(simple_error(block_start, source, "expected '{'"));
    }

    let mut index = block_start + 1;
    let mut body = Vec::new();
    loop {
        skip_ws_and_comments(source, &mut index)?;
        if index >= source.len() {
            return Err(simple_error(
                source.len().saturating_sub(1),
                source,
                "unterminated block",
            ));
        }
        if source.as_bytes()[index] == b'}' {
            return Ok((
                index + 1,
                Pos::from_offsets(source, file_path, block_start, index + 1),
                body,
            ));
        }

        let stmt_start = index;
        index = consume_statement(source, stmt_start)?;
        body.push(SyntaxSnippet::new(
            Pos::from_offsets(source, file_path, stmt_start, index),
            source[stmt_start..index].to_string(),
        ));
    }
}

fn scan_expression_block<'src>(
    source: &'src str,
    file_path: &str,
    block_start: usize,
) -> Result<(usize, Pos, Vec<SyntaxSnippet>), Simple<'src, char>> {
    if block_start >= source.len() || source.as_bytes()[block_start] != b'{' {
        return Err(simple_error(block_start, source, "expected '{'"));
    }

    let mut index = block_start + 1;
    let mut snippets = Vec::new();
    loop {
        skip_ws_and_comments(source, &mut index)?;
        if index >= source.len() {
            return Err(simple_error(
                source.len().saturating_sub(1),
                source,
                "unterminated block",
            ));
        }
        if source.as_bytes()[index] == b'}' {
            return Ok((
                index + 1,
                Pos::from_offsets(source, file_path, block_start, index + 1),
                snippets,
            ));
        }

        let expr_start = index;
        index = consume_semicolon_terminated(source, expr_start)?;
        let expr_end = index.saturating_sub(1);
        snippets.push(SyntaxSnippet::new(
            Pos::from_offsets(source, file_path, expr_start, expr_end),
            source[expr_start..expr_end].trim().to_string(),
        ));
    }
}

fn parse_identifier_snippet<'src>(
    source: &'src str,
    file_path: &str,
    index: &mut usize,
) -> Result<SyntaxSnippet, Simple<'src, char>> {
    let start = *index;
    *index = consume_identifier(source, *index)?;
    Ok(SyntaxSnippet::new(
        Pos::from_offsets(source, file_path, start, *index),
        source[start..*index].to_string(),
    ))
}

fn consume_statement<'src>(source: &'src str, start: usize) -> Result<usize, Simple<'src, char>> {
    let mut index = start;
    skip_ws_and_comments(source, &mut index)?;
    if index >= source.len() {
        return Err(simple_error(start, source, "expected statement"));
    }

    if source.as_bytes()[index] == b'{' {
        return consume_balanced(source, index, b'{', b'}');
    }
    if source.as_bytes()[index] == b'@' {
        index += 1;
        return consume_statement(source, index);
    }
    if starts_with_keyword(source, index, "if") {
        return consume_if_statement(source, index);
    }
    if starts_with_keyword(source, index, "while") {
        return consume_while_statement(source, index);
    }
    if starts_with_keyword(source, index, "loop") {
        index += "loop".len();
        return consume_statement(source, index);
    }
    if starts_with_keyword(source, index, "for") {
        return consume_for_statement(source, index);
    }
    if starts_with_keyword(source, index, "atomic") {
        index += "atomic".len();
        return consume_statement(source, index);
    }
    if starts_with_keyword(source, index, "await") || starts_with_keyword(source, index, "wait") {
        return consume_wait_statement(source, index);
    }

    consume_semicolon_terminated(source, index)
}

fn consume_if_statement<'src>(source: &'src str, start: usize) -> Result<usize, Simple<'src, char>> {
    let mut index = start + "if".len();
    skip_ws_and_comments(source, &mut index)?;
    index = consume_expression_until_block(source, index)?;
    skip_ws_and_comments(source, &mut index)?;
    index = consume_balanced(source, index, b'{', b'}')?;
    skip_ws_and_comments(source, &mut index)?;
    if starts_with_keyword(source, index, "else") {
        index += "else".len();
        skip_ws_and_comments(source, &mut index)?;
        if starts_with_keyword(source, index, "if") {
            consume_if_statement(source, index)
        } else {
            consume_balanced(source, index, b'{', b'}')
        }
    } else {
        Ok(index)
    }
}

fn consume_while_statement<'src>(
    source: &'src str,
    start: usize,
) -> Result<usize, Simple<'src, char>> {
    let mut index = start + "while".len();
    skip_ws_and_comments(source, &mut index)?;
    index = consume_expression_until_block(source, index)?;
    skip_ws_and_comments(source, &mut index)?;
    consume_balanced(source, index, b'{', b'}')
}

fn consume_for_statement<'src>(source: &'src str, start: usize) -> Result<usize, Simple<'src, char>> {
    let mut index = start + "for".len();
    skip_ws_and_comments(source, &mut index)?;
    index = consume_identifier(source, index)?;
    skip_ws_and_comments(source, &mut index)?;
    consume_keyword(source, &mut index, "in")?;
    skip_ws_and_comments(source, &mut index)?;
    index = consume_expression_until_statement(source, index)?;
    skip_ws_and_comments(source, &mut index)?;
    consume_statement(source, index)
}

fn consume_wait_statement<'src>(
    source: &'src str,
    start: usize,
) -> Result<usize, Simple<'src, char>> {
    let mut index = start;
    if starts_with_keyword(source, index, "await") {
        index += "await".len();
    } else {
        index += "wait".len();
    }
    skip_ws_and_comments(source, &mut index)?;

    if starts_with_keyword(source, index, "first") || starts_with_keyword(source, index, "seq") {
        if starts_with_keyword(source, index, "first") {
            index += "first".len();
        } else {
            index += "seq".len();
        }
        skip_ws_and_comments(source, &mut index)?;
        return consume_waiting_block(source, index);
    }

    consume_waiting_case(source, index)
}

fn consume_waiting_block<'src>(source: &'src str, start: usize) -> Result<usize, Simple<'src, char>> {
    if source.as_bytes().get(start) != Some(&b'{') {
        return Err(simple_error(start, source, "expected '{'"));
    }
    let mut index = start + 1;
    loop {
        skip_ws_and_comments(source, &mut index)?;
        if index >= source.len() {
            return Err(simple_error(
                source.len().saturating_sub(1),
                source,
                "unterminated block",
            ));
        }
        if source.as_bytes()[index] == b'}' {
            return Ok(index + 1);
        }
        index = consume_waiting_case(source, index)?;
    }
}

fn consume_waiting_case<'src>(source: &'src str, start: usize) -> Result<usize, Simple<'src, char>> {
    let mut index = consume_wait_case_rule(source, start)?;
    skip_ws_and_comments(source, &mut index)?;
    if source.as_bytes().get(index) == Some(&b';') {
        return Ok(index + 1);
    }
    if source[index..].starts_with("=>") {
        index += 2;
        skip_ws_and_comments(source, &mut index)?;
        return consume_statement(source, index);
    }
    Err(simple_error(index, source, "expected ';' or '=>'"))
}

fn consume_wait_case_rule<'src>(
    source: &'src str,
    start: usize,
) -> Result<usize, Simple<'src, char>> {
    let mut index = start;
    skip_ws_and_comments(source, &mut index)?;
    if starts_with_keyword(source, index, "receive") {
        index += "receive".len();
        skip_ws_and_comments(source, &mut index)?;
        if source.as_bytes().get(index) != Some(&b'(') {
            index = consume_object_identifier_path(source, index)?;
            skip_ws_and_comments(source, &mut index)?;
        }
        return consume_balanced(source, index, b'(', b')');
    }
    consume_expression_until_any(source, index, &["=>", ";"])
}

fn consume_expression_until_block<'src>(
    source: &'src str,
    start: usize,
) -> Result<usize, Simple<'src, char>> {
    consume_expression_until_any(source, start, &["{"])
}

fn consume_expression_until_statement<'src>(
    source: &'src str,
    start: usize,
) -> Result<usize, Simple<'src, char>> {
    consume_expression_until_any(source, start, &["{"])
}

fn consume_semicolon_terminated<'src>(
    source: &'src str,
    start: usize,
) -> Result<usize, Simple<'src, char>> {
    let end = consume_expression_until_any(source, start, &[";"])?;
    if source.as_bytes().get(end) != Some(&b';') {
        return Err(simple_error(end, source, "expected ';'"));
    }
    Ok(end + 1)
}

fn consume_object_identifier_path<'src>(
    source: &'src str,
    mut index: usize,
) -> Result<usize, Simple<'src, char>> {
    index = consume_identifier(source, index)?;
    loop {
        let mut after = index;
        skip_ws_and_comments(source, &mut after)?;
        if source.as_bytes().get(after) != Some(&b'.') {
            return Ok(index);
        }
        after += 1;
        skip_ws_and_comments(source, &mut after)?;
        index = consume_identifier(source, after)?;
    }
}

fn consume_expression_until_any<'src>(
    source: &'src str,
    start: usize,
    delimiters: &[&str],
) -> Result<usize, Simple<'src, char>> {
    let mut index = start;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    loop {
        if index >= source.len() {
            return Err(simple_error(
                source.len().saturating_sub(1),
                source,
                "unterminated expression",
            ));
        }
        if source[index..].starts_with("//") {
            while index < source.len() && source.as_bytes()[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if source[index..].starts_with("/*") {
            index += 2;
            while index + 1 < source.len() && &source[index..index + 2] != "*/" {
                index += 1;
            }
            if index + 1 >= source.len() {
                return Err(simple_error(
                    source.len().saturating_sub(1),
                    source,
                    "unterminated block comment",
                ));
            }
            index += 2;
            continue;
        }
        if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
            for delimiter in delimiters {
                if source[index..].starts_with(delimiter) {
                    return Ok(index);
                }
            }
        }

        match source.as_bytes()[index] {
            b'"' => index = consume_string(source, index)?,
            b'(' => {
                paren_depth += 1;
                index += 1;
            }
            b')' => {
                paren_depth = paren_depth.saturating_sub(1);
                index += 1;
            }
            b'[' => {
                bracket_depth += 1;
                index += 1;
            }
            b']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                index += 1;
            }
            b'{' => {
                brace_depth += 1;
                index += 1;
            }
            b'}' => {
                if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                    return Err(simple_error(index, source, "unexpected '}'"));
                }
                brace_depth = brace_depth.saturating_sub(1);
                index += 1;
            }
            _ => index += 1,
        }
    }
}

fn parse_balanced_snippet<'src>(
    source: &'src str,
    file_path: &str,
    index: &mut usize,
    open: u8,
    close: u8,
) -> Result<SyntaxSnippet, Simple<'src, char>> {
    let start = *index;
    *index = consume_balanced(source, *index, open, close)?;
    Ok(SyntaxSnippet::new(
        Pos::from_offsets(source, file_path, start, *index),
        source[start..*index].to_string(),
    ))
}

fn parse_block_kind<'src>(
    source: &'src str,
    index: usize,
) -> Result<SyntaxBlockKind, Simple<'src, char>> {
    let keyword_index = if starts_with_keyword(source, index, "@private") {
        let mut inner = index + "@private".len();
        skip_ws_and_comments(source, &mut inner)?;
        inner
    } else {
        index
    };

    if starts_with_keyword(source, keyword_index, "import") {
        Ok(SyntaxBlockKind::Import)
    } else if starts_with_keyword(source, keyword_index, "main") {
        Ok(SyntaxBlockKind::Main)
    } else if starts_with_keyword(source, keyword_index, "shared") {
        Ok(SyntaxBlockKind::Global)
    } else if starts_with_keyword(source, keyword_index, "always") {
        Ok(SyntaxBlockKind::Always)
    } else if starts_with_keyword(source, keyword_index, "never") {
        Ok(SyntaxBlockKind::Never)
    } else if starts_with_keyword(source, keyword_index, "check") {
        Ok(SyntaxBlockKind::Check)
    } else if starts_with_keyword(source, keyword_index, "program") {
        Ok(SyntaxBlockKind::Program)
    } else if starts_with_keyword(source, keyword_index, "fn") {
        Ok(SyntaxBlockKind::Function)
    } else {
        Err(simple_error(
            keyword_index,
            source,
            "expected a top-level block",
        ))
    }
}

fn skip_ws_and_comments<'src>(
    source: &'src str,
    index: &mut usize,
) -> Result<(), Simple<'src, char>> {
    loop {
        while *index < source.len() && source.as_bytes()[*index].is_ascii_whitespace() {
            *index += 1;
        }

        if *index >= source.len() {
            break;
        }

        if source[*index..].starts_with("//") {
            *index += 2;
            while *index < source.len() && source.as_bytes()[*index] != b'\n' {
                *index += 1;
            }
            continue;
        }

        if source[*index..].starts_with("/*") {
            *index += 2;
            while *index + 1 < source.len() && &source[*index..*index + 2] != "*/" {
                *index += 1;
            }
            if *index + 1 >= source.len() {
                return Err(simple_error(
                    source.len().saturating_sub(1),
                    source,
                    "unterminated block comment",
                ));
            }
            *index += 2;
            continue;
        }

        break;
    }

    Ok(())
}

fn skip_inline_ws_and_comments(text: &str, index: &mut usize) -> Result<(), AlthreadError> {
    loop {
        while *index < text.len() && text.as_bytes()[*index].is_ascii_whitespace() {
            *index += 1;
        }

        if *index >= text.len() {
            break;
        }

        if text[*index..].starts_with("//") {
            *index += 2;
            while *index < text.len() && text.as_bytes()[*index] != b'\n' {
                *index += 1;
            }
            continue;
        }

        if text[*index..].starts_with("/*") {
            *index += 2;
            while *index + 1 < text.len() && &text[*index..*index + 2] != "*/" {
                *index += 1;
            }
            if *index + 1 >= text.len() {
                return Err(AlthreadError::new(
                    ErrorType::SyntaxError,
                    None,
                    "unterminated block comment".to_string(),
                ));
            }
            *index += 2;
            continue;
        }

        break;
    }

    Ok(())
}

fn consume_keyword<'src>(
    source: &'src str,
    index: &mut usize,
    keyword: &str,
) -> Result<(), Simple<'src, char>> {
    if starts_with_keyword(source, *index, keyword) {
        *index += keyword.len();
        Ok(())
    } else {
        Err(simple_error(
            *index,
            source,
            &format!("expected '{keyword}'"),
        ))
    }
}

fn starts_with_keyword(source: &str, index: usize, keyword: &str) -> bool {
    if index > source.len() || !source[index..].starts_with(keyword) {
        return false;
    }

    let end = index + keyword.len();
    if keyword == "@private" {
        return end >= source.len()
            || !is_ident_char(source.as_bytes().get(end).copied().unwrap_or_default());
    }

    end >= source.len() || !is_ident_char(source.as_bytes().get(end).copied().unwrap_or_default())
}

fn consume_identifier<'src>(
    source: &'src str,
    mut index: usize,
) -> Result<usize, Simple<'src, char>> {
    if index >= source.len() || !source.as_bytes()[index].is_ascii_alphabetic() {
        return Err(simple_error(index, source, "expected identifier"));
    }
    index += 1;
    while index < source.len() && is_ident_char(source.as_bytes()[index]) {
        index += 1;
    }
    Ok(index)
}

fn consume_until_block<'src>(
    source: &'src str,
    mut index: usize,
) -> Result<usize, Simple<'src, char>> {
    let mut paren_depth = 0usize;
    while index < source.len() {
        if source[index..].starts_with("//") {
            while index < source.len() && source.as_bytes()[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if source[index..].starts_with("/*") {
            index += 2;
            while index + 1 < source.len() && &source[index..index + 2] != "*/" {
                index += 1;
            }
            if index + 1 >= source.len() {
                return Err(simple_error(
                    source.len().saturating_sub(1),
                    source,
                    "unterminated block comment",
                ));
            }
            index += 2;
            continue;
        }
        match source.as_bytes()[index] {
            b'"' => index = consume_string(source, index)?,
            b'(' => {
                paren_depth += 1;
                index += 1;
            }
            b')' => {
                paren_depth = paren_depth.saturating_sub(1);
                index += 1;
            }
            b'{' if paren_depth == 0 => return Ok(index),
            _ => index += 1,
        }
    }

    Err(simple_error(
        source.len().saturating_sub(1),
        source,
        "expected a function body",
    ))
}

fn consume_balanced<'src>(
    source: &'src str,
    mut index: usize,
    open: u8,
    close: u8,
) -> Result<usize, Simple<'src, char>> {
    if index >= source.len() || source.as_bytes()[index] != open {
        return Err(simple_error(
            index,
            source,
            &format!("expected '{}'", open as char),
        ));
    }

    let mut depth = 0usize;
    while index < source.len() {
        if source[index..].starts_with("//") {
            while index < source.len() && source.as_bytes()[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if source[index..].starts_with("/*") {
            index += 2;
            while index + 1 < source.len() && &source[index..index + 2] != "*/" {
                index += 1;
            }
            if index + 1 >= source.len() {
                return Err(simple_error(
                    source.len().saturating_sub(1),
                    source,
                    "unterminated block comment",
                ));
            }
            index += 2;
            continue;
        }

        match source.as_bytes()[index] {
            b'"' => index = consume_string(source, index)?,
            ch if ch == open => {
                depth += 1;
                index += 1;
            }
            ch if ch == close => {
                depth -= 1;
                index += 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => index += 1,
        }
    }

    Err(simple_error(
        source.len().saturating_sub(1),
        source,
        &format!("unterminated '{}{}' block", open as char, close as char),
    ))
}

fn consume_string<'src>(source: &'src str, mut index: usize) -> Result<usize, Simple<'src, char>> {
    index += 1;
    while index < source.len() {
        if source.as_bytes()[index] == b'"' {
            return Ok(index + 1);
        }
        index += 1;
    }

    Err(simple_error(
        source.len().saturating_sub(1),
        source,
        "unterminated string literal",
    ))
}

fn is_ident_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn consume_word(text: &str, index: &mut usize, word: &str) -> bool {
    if *index > text.len() || !text[*index..].starts_with(word) {
        return false;
    }
    let end = *index + word.len();
    if end < text.len() && is_ident_char(text.as_bytes()[end]) {
        return false;
    }
    *index = end;
    true
}

fn starts_with_word_local(text: &str, index: usize, word: &str) -> bool {
    if index > text.len() || !text[index..].starts_with(word) {
        return false;
    }
    let end = index + word.len();
    end >= text.len() || !is_ident_char(text.as_bytes()[end])
}

fn consume_identifier_local(text: &str, mut index: usize) -> Option<usize> {
    if index >= text.len() || !text.as_bytes()[index].is_ascii_alphabetic() {
        return None;
    }
    index += 1;
    while index < text.len() && is_ident_char(text.as_bytes()[index]) {
        index += 1;
    }
    Some(index)
}

fn consume_number_local(text: &str, mut index: usize) -> Option<usize> {
    if index >= text.len() || !text.as_bytes()[index].is_ascii_digit() {
        return None;
    }
    if text[index..].starts_with("0x") || text[index..].starts_with("0X") {
        index += 2;
        while index < text.len() && text.as_bytes()[index].is_ascii_hexdigit() {
            index += 1;
        }
        return Some(index);
    }
    if text[index..].starts_with("0b") || text[index..].starts_with("0B") {
        index += 2;
        while index < text.len() && matches!(text.as_bytes()[index], b'0' | b'1') {
            index += 1;
        }
        return Some(index);
    }
    while index < text.len() && text.as_bytes()[index].is_ascii_digit() {
        index += 1;
    }
    if text.as_bytes().get(index) == Some(&b'.')
        && text
            .as_bytes()
            .get(index + 1)
            .copied()
            .is_some_and(|b| b.is_ascii_digit())
    {
        index += 1;
        while index < text.len() && text.as_bytes()[index].is_ascii_digit() {
            index += 1;
        }
    }
    Some(index)
}

fn consume_import_segment(text: &str, mut index: usize) -> Option<usize> {
    let first = text.as_bytes().get(index).copied()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    index += 1;
    while let Some(byte) = text.as_bytes().get(index).copied() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-') {
            index += 1;
        } else {
            break;
        }
    }
    Some(index)
}

fn split_top_level_range(text: &str) -> Result<Option<(usize, usize)>, AlthreadError> {
    let mut index = 0;
    let mut paren = 0usize;
    let mut brace = 0usize;
    let mut bracket = 0usize;
    while index + 1 < text.len() {
        if text[index..].starts_with("//") {
            while index < text.len() && text.as_bytes()[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if text[index..].starts_with("/*") {
            index += 2;
            while index + 1 < text.len() && &text[index..index + 2] != "*/" {
                index += 1;
            }
            index = (index + 2).min(text.len());
            continue;
        }
        match text.as_bytes()[index] {
            b'"' => {
                index += 1;
                while index < text.len() && text.as_bytes()[index] != b'"' {
                    index += 1;
                }
                index = (index + 1).min(text.len());
            }
            b'(' => {
                paren += 1;
                index += 1;
            }
            b')' => {
                paren = paren.saturating_sub(1);
                index += 1;
            }
            b'{' => {
                brace += 1;
                index += 1;
            }
            b'}' => {
                brace = brace.saturating_sub(1);
                index += 1;
            }
            b'[' => {
                bracket += 1;
                index += 1;
            }
            b']' => {
                bracket = bracket.saturating_sub(1);
                index += 1;
            }
            b'.' if paren == 0 && brace == 0 && bracket == 0 && text[index + 1..].starts_with('.') => {
                return Ok(Some((index, index + 2)));
            }
            _ => index += 1,
        }
    }
    Ok(None)
}

fn find_top_level_block_start(text: &str, start: usize) -> Option<usize> {
    let mut index = start;
    let mut paren_depth = 0usize;
    while index < text.len() {
        match text.as_bytes()[index] {
            b'"' => {
                index += 1;
                while index < text.len() && text.as_bytes()[index] != b'"' {
                    index += 1;
                }
                index += 1;
            }
            b'(' => {
                paren_depth += 1;
                index += 1;
            }
            b')' => {
                paren_depth = paren_depth.saturating_sub(1);
                index += 1;
            }
            b'{' if paren_depth == 0 => return Some(index),
            _ => index += 1,
        }
    }
    None
}

fn consume_balanced_block(text: &str, start: usize, open: char, close: char) -> Option<usize> {
    let open = open as u8;
    let close = close as u8;
    if text.as_bytes().get(start) != Some(&open) {
        return None;
    }
    let mut depth = 0usize;
    let mut index = start;
    while index < text.len() {
        match text.as_bytes()[index] {
            b'"' => {
                index += 1;
                while index < text.len() && text.as_bytes()[index] != b'"' {
                    index += 1;
                }
                index += 1;
            }
            ch if ch == open => {
                depth += 1;
                index += 1;
            }
            ch if ch == close => {
                depth = depth.saturating_sub(1);
                index += 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => index += 1,
        }
    }
    None
}

fn expect_char(
    text: &str,
    source: &str,
    file_path: &str,
    base_offset: usize,
    index: &mut usize,
    expected: char,
) -> Result<(), AlthreadError> {
    skip_inline_ws_and_comments(text, index)?;
    match text[*index..].chars().next() {
        Some(found) if found == expected => {
            *index += found.len_utf8();
            Ok(())
        }
        Some(found) => Err(scan_error(
            source,
            file_path,
            base_offset + *index,
            base_offset + *index + found.len_utf8(),
            &format!("expected '{expected}', found '{found}'"),
        )),
        None => Err(scan_error(
            source,
            file_path,
            base_offset + text.len().saturating_sub(1),
            base_offset + text.len(),
            &format!("expected '{expected}'"),
        )),
    }
}

fn scan_error(
    source: &str,
    file_path: &str,
    start: usize,
    end: usize,
    message: &str,
) -> AlthreadError {
    AlthreadError::new(
        ErrorType::SyntaxError,
        Some(Pos::from_offsets(source, file_path, start, end)),
        message.to_string(),
    )
}

fn simple_error<'src>(index: usize, source: &'src str, message: &str) -> Simple<'src, char> {
    let _ = message;
    let found = source[index..].chars().next();
    Simple::new(
        found.map(Into::into),
        SimpleSpan::new((), index..(index + found.map_or(1, char::len_utf8))),
    )
}

fn map_chumsky_errors<'src>(
    source: &'src str,
    file_path: &str,
    errs: Vec<Simple<'src, char>>,
) -> AlthreadError {
    let err = errs.into_iter().next().expect("chumsky returned no errors");
    let span = err.span();
    AlthreadError::new(
        ErrorType::SyntaxError,
        Some(Pos::from_offsets(source, file_path, span.start, span.end)),
        err.to_string(),
    )
}
