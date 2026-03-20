pub mod block;
pub mod condition_block;
pub mod display;
pub mod import_block;
pub mod node;
pub mod statement;
pub mod token;

use std::{
    collections::HashMap,
    fmt::{self, Formatter},
};

use block::Block;
use condition_block::ConditionBlock;
use display::{AstDisplay, Prefix};
use import_block::ImportBlock;
use node::Node;
use statement::{
    assignment::{binary_assignment::BinaryAssignment, Assignment},
    atomic::Atomic,
    break_loop::{BreakLoopControl, BreakLoopType},
    declaration::Declaration,
    expression::{primary_expression::PrimaryExpression, Expression, SideEffectExpression},
    fn_return::FnReturn,
    for_control::ForControl,
    if_control::IfControl,
    label::LabelStatement,
    loop_control::LoopControl,
    receive::ReceiveStatement,
    wait::{Wait, WaitingBlockKind},
    waiting_case::{WaitingBlockCase, WaitingBlockCaseRule},
    while_control::WhileControl,
    Statement,
};
use token::{
    args_list::ArgsList, binary_assignment_operator::BinaryAssignmentOperator,
    condition_keyword::ConditionKeyword, datatype::DataType,
    declaration_keyword::DeclarationKeyword, identifier::Identifier,
    object_identifier::ObjectIdentifier,
};

use crate::{
    checker::ltl::ast::parse_ltl_expression_with_chumsky,
    error::{AlthreadError, AlthreadResult, ErrorType, Pos},
    parser::{
        parse_args_list_with_chumsky, parse_channel_declaration_with_chumsky,
        parse_datatype_with_chumsky, parse_expression_with_chumsky, parse_fn_call_with_chumsky,
        parse_import_block_with_chumsky, parse_list_expression_with_chumsky,
        parse_object_identifier_with_chumsky, parse_run_call_with_chumsky,
        parse_send_call_with_chumsky, parse_side_effect_expression_with_chumsky,
        parse_statement_block_with_chumsky,
        syntax::{SyntaxBlockDetail, SyntaxBlockKind, SyntaxProgram, SyntaxSnippet},
    },
};

use crate::checker::ltl::ast::CheckBlock;

#[derive(Debug)]
pub struct Ast {
    pub process_blocks: HashMap<String, (Node<ArgsList>, Node<Block>, bool)>,
    pub condition_blocks: HashMap<ConditionKeyword, Node<ConditionBlock>>,
    pub check_blocks: Vec<Node<CheckBlock>>,
    pub global_block: Option<Node<Block>>,
    pub function_blocks: HashMap<String, (Node<ArgsList>, DataType, Node<Block>, bool)>,
    pub import_block: Option<Node<ImportBlock>>,
}

impl Ast {
    pub fn new() -> Self {
        Self {
            process_blocks: HashMap::new(),
            condition_blocks: HashMap::new(),
            check_blocks: Vec::new(),
            global_block: None,
            function_blocks: HashMap::new(),
            import_block: None,
        }
    }
    pub fn from_syntax(
        source: &str,
        syntax: SyntaxProgram,
        filepath: &str,
    ) -> AlthreadResult<Self> {
        let mut ast = Self::new();
        for block in syntax.blocks {
            match &block.detail {
                SyntaxBlockDetail::Opaque => {
                    if block.kind != SyntaxBlockKind::Import {
                        return Err(AlthreadError::new(
                            ErrorType::SyntaxError,
                            Some(block.pos.clone()),
                            "Unexpected opaque top-level block in chumsky parser".to_string(),
                        ));
                    }
                    let snippet = SyntaxSnippet::new(block.pos.clone(), block.text.clone());
                    ast.import_block =
                        Some(parse_import_block_with_chumsky(source, &snippet, filepath)?);
                }
                SyntaxBlockDetail::Main { body_pos, body } => {
                    let main_block =
                        build_statement_block(source, body, body_pos.clone(), filepath)?;
                    ast.process_blocks.insert(
                        "main".to_string(),
                        (Node::<ArgsList>::new(), main_block, false),
                    );
                }
                SyntaxBlockDetail::Global { body_pos, body } => {
                    ast.global_block = Some(build_statement_block(
                        source,
                        body,
                        body_pos.clone(),
                        filepath,
                    )?);
                }
                SyntaxBlockDetail::Condition { body_pos, body } => {
                    let keyword = match block.kind {
                        SyntaxBlockKind::Always => ConditionKeyword::Always,
                        SyntaxBlockKind::Never => ConditionKeyword::Never,
                        _ => {
                            return Err(AlthreadError::new(
                                ErrorType::SyntaxError,
                                Some(block.pos.clone()),
                                "Unexpected condition block kind".to_string(),
                            ));
                        }
                    };
                    let condition_block =
                        build_expression_block(source, body, body_pos.clone(), filepath)?;
                    ast.condition_blocks.insert(keyword, condition_block);
                }
                SyntaxBlockDetail::Check { formulas, .. } => {
                    ast.check_blocks.push(build_check_block(
                        source,
                        formulas,
                        block.pos.clone(),
                        filepath,
                    )?);
                }
                SyntaxBlockDetail::Program {
                    is_private,
                    name,
                    args,
                    body_pos,
                    body,
                } => {
                    let args_list = parse_args_list_with_chumsky(source, args, filepath)?;
                    let program_block =
                        build_statement_block(source, body, body_pos.clone(), filepath)?;
                    ast.process_blocks
                        .insert(name.text.clone(), (args_list, program_block, *is_private));
                }
                SyntaxBlockDetail::Function {
                    is_private,
                    name,
                    args,
                    return_type,
                    body_pos,
                    body,
                } => {
                    let args_list = parse_args_list_with_chumsky(source, args, filepath)?;
                    let return_datatype =
                        parse_datatype_with_chumsky(source, return_type, filepath)?.value;
                    let function_block =
                        build_statement_block(source, body, body_pos.clone(), filepath)?;
                    if ast.function_blocks.contains_key(&name.text) {
                        return Err(AlthreadError::new(
                            ErrorType::FunctionAlreadyDefined,
                            Some(function_block.pos.clone()),
                            format!("Function '{}' is already defined", name.text),
                        ));
                    }
                    ast.function_blocks.insert(
                        name.text.clone(),
                        (args_list, return_datatype, function_block, *is_private),
                    );
                }
            }
        }
        Ok(ast)
    }

    pub fn diff_summary(&self, other: &Self) -> Option<String> {
        let lhs = self.canonical_repr();
        let rhs = other.canonical_repr();
        if lhs == rhs {
            return None;
        }

        let mut lhs_lines = lhs.lines();
        let mut rhs_lines = rhs.lines();
        for line_number in 1.. {
            match (lhs_lines.next(), rhs_lines.next()) {
                (Some(left), Some(right)) if left == right => continue,
                (Some(left), Some(right)) => {
                    return Some(format!(
                        "AST mismatch at line {line_number}: expected `{left}`, got `{right}`"
                    ));
                }
                (Some(left), None) => {
                    return Some(format!(
                        "AST mismatch at line {line_number}: unexpected extra line `{left}`"
                    ));
                }
                (None, Some(right)) => {
                    return Some(format!(
                        "AST mismatch at line {line_number}: missing line, got `{right}`"
                    ));
                }
                (None, None) => break,
            }
        }

        Some("AST mismatch".to_string())
    }

    fn canonical_repr(&self) -> String {
        let mut out = String::new();
        if let Some(import_block) = &self.import_block {
            out.push_str("import\n");
            out.push_str(&format!("{import_block:?}\n"));
        }
        if let Some(global_block) = &self.global_block {
            out.push_str("shared\n");
            out.push_str(&format!("{global_block:?}\n"));
        }

        let mut condition_entries = self.condition_blocks.iter().collect::<Vec<_>>();
        condition_entries.sort_by_key(|(keyword, _)| format!("{keyword:?}"));
        for (keyword, block) in condition_entries {
            out.push_str(&format!("condition:{keyword:?}\n{block:?}\n"));
        }

        for check_block in &self.check_blocks {
            out.push_str(&format!("check:{check_block:?}\n"));
        }

        let mut process_entries = self.process_blocks.iter().collect::<Vec<_>>();
        process_entries.sort_by_key(|(name, _)| (*name).clone());
        for (name, value) in process_entries {
            out.push_str(&format!("process:{name}:{value:?}\n"));
        }

        let mut function_entries = self.function_blocks.iter().collect::<Vec<_>>();
        function_entries.sort_by_key(|(name, _)| (*name).clone());
        for (name, value) in function_entries {
            out.push_str(&format!("function:{name}:{value:?}\n"));
        }

        out
    }
}

fn build_statement_block(
    source: &str,
    snippets: &[SyntaxSnippet],
    pos: Pos,
    filepath: &str,
) -> AlthreadResult<Node<Block>> {
    let children = snippets
        .iter()
        .map(|snippet| parse_statement_strict(source, snippet, filepath))
        .collect::<AlthreadResult<Vec<_>>>()?;
    Ok(Node {
        value: Block { children },
        pos,
    })
}

fn build_expression_block(
    source: &str,
    snippets: &[SyntaxSnippet],
    pos: Pos,
    filepath: &str,
) -> AlthreadResult<Node<ConditionBlock>> {
    let children = snippets
        .iter()
        .map(|snippet| {
            let expr = parse_expression_snippet(source, snippet, filepath)?;
            Ok(Node {
                pos: Pos::from_offsets(source, filepath, snippet.pos.start, snippet.pos.end + 1),
                value: Expression::Primary(Node {
                    pos: expr.pos.clone(),
                    value: PrimaryExpression::Expression(Box::new(expr)),
                }),
            })
        })
        .collect::<AlthreadResult<Vec<_>>>()?;
    Ok(Node {
        value: ConditionBlock { children },
        pos,
    })
}

fn build_check_block(
    source: &str,
    snippets: &[SyntaxSnippet],
    pos: Pos,
    filepath: &str,
) -> AlthreadResult<Node<CheckBlock>> {
    let formulas = snippets
        .iter()
        .map(|snippet| parse_ltl_expression_with_chumsky(source, snippet, filepath))
        .collect::<AlthreadResult<Vec<_>>>()?;
    Ok(Node {
        value: CheckBlock { formulas },
        pos,
    })
}

fn parse_expression_snippet(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Expression>> {
    parse_expression_with_chumsky(source, snippet, filepath)
}

fn parse_side_effect_expression_snippet(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<SideEffectExpression>> {
    parse_side_effect_expression_with_chumsky(source, snippet, filepath)
}

fn parse_statement_with_chumsky(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Option<Node<Statement>>> {
    let mut index = 0;
    skip_inline_ws(&snippet.text, &mut index);

    if snippet.text[index..].starts_with('{') {
        let (_, block) = parse_code_block_snippet(source, snippet, filepath, index)?;
        return Ok(Some(Node {
            pos: block.pos.clone(),
            value: Statement::Block(block),
        }));
    }

    if consume_keyword(&snippet.text, &mut index, "let")
        || consume_keyword(&snippet.text, &mut index, "const")
    {
        return parse_declaration_statement(source, snippet, filepath).map(Some);
    }

    index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if consume_keyword(&snippet.text, &mut index, "return") {
        return parse_return_statement(source, snippet, filepath).map(Some);
    }

    index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if consume_keyword(&snippet.text, &mut index, "label") {
        return parse_label_statement(source, snippet, filepath).map(Some);
    }

    index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if consume_keyword(&snippet.text, &mut index, "break")
        || consume_keyword(&snippet.text, &mut index, "continue")
    {
        return parse_break_loop_statement(source, snippet, filepath).map(Some);
    }

    index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if consume_keyword(&snippet.text, &mut index, "if") {
        return parse_if_statement(source, snippet, filepath).map(Some);
    }

    index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if consume_keyword(&snippet.text, &mut index, "while") {
        return parse_while_statement(source, snippet, filepath).map(Some);
    }

    index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if consume_keyword(&snippet.text, &mut index, "loop") {
        return parse_loop_statement(source, snippet, filepath).map(Some);
    }

    index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if consume_keyword(&snippet.text, &mut index, "for") {
        return parse_for_statement(source, snippet, filepath).map(Some);
    }

    index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if consume_keyword(&snippet.text, &mut index, "atomic")
        || snippet.text.trim_start().starts_with('@')
    {
        return parse_atomic_statement(source, snippet, filepath).map(Some);
    }

    index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if consume_keyword(&snippet.text, &mut index, "run") {
        return parse_run_statement(source, snippet, filepath).map(Some);
    }

    index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if consume_keyword(&snippet.text, &mut index, "send") {
        return parse_send_statement(source, snippet, filepath).map(Some);
    }

    index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if consume_keyword(&snippet.text, &mut index, "channel") {
        return parse_channel_declaration_statement(source, snippet, filepath).map(Some);
    }

    index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if consume_keyword(&snippet.text, &mut index, "await")
        || consume_keyword(&snippet.text, &mut index, "wait")
    {
        return parse_wait_statement(source, snippet, filepath).map(Some);
    }

    if has_top_level_assignment(&snippet.text) {
        return parse_assignment_statement(source, snippet, filepath).map(Some);
    }

    if looks_like_call_statement(&snippet.text) {
        return parse_fn_call_statement(source, snippet, filepath).map(Some);
    }

    Ok(None)
}

fn parse_declaration_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    let mut index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    let keyword_start = index;
    let keyword = if consume_keyword(&snippet.text, &mut index, "let") {
        DeclarationKeyword::Let
    } else if consume_keyword(&snippet.text, &mut index, "const") {
        DeclarationKeyword::Const
    } else {
        return Err(snippet_error(
            source,
            filepath,
            snippet.pos.start + index,
            snippet.pos.start + (index + 1).min(snippet.text.len()),
            "expected declaration keyword",
        ));
    };
    let keyword_node = Node {
        pos: Pos::from_offsets(
            source,
            filepath,
            snippet.pos.start + keyword_start,
            snippet.pos.start + index,
        ),
        value: keyword,
    };

    let identifier = parse_object_identifier_node(source, filepath, snippet, &mut index)?;

    skip_inline_ws(&snippet.text, &mut index);
    let datatype = if snippet.text.as_bytes().get(index) == Some(&b':') {
        index += 1;
        let datatype_start = index;
        let datatype_end = find_top_level_char(&snippet.text, datatype_start, &['=', ';'])
            .unwrap_or(snippet.text.len());
        let datatype_snippet =
            sub_snippet(source, filepath, snippet, datatype_start, datatype_end)?;
        index = datatype_end;
        Some(parse_datatype_with_chumsky(
            source,
            &datatype_snippet,
            filepath,
        )?)
    } else {
        None
    };

    skip_inline_ws(&snippet.text, &mut index);
    let value = if snippet.text.as_bytes().get(index) == Some(&b'=') {
        index += 1;
        let value_start = index;
        let value_end = find_statement_semicolon(&snippet.text, value_start).ok_or_else(|| {
            snippet_error(
                source,
                filepath,
                snippet.pos.start + value_start,
                snippet.pos.end,
                "expected ';' after declaration value",
            )
        })?;
        let value_snippet = sub_snippet(source, filepath, snippet, value_start, value_end)?;
        index = value_end;
        Some(parse_side_effect_expression_snippet(
            source,
            &value_snippet,
            filepath,
        )?)
    } else {
        None
    };

    let value_end = index;
    let statement_end = expect_statement_end(source, filepath, snippet, &mut index)?;
    let statement_pos = Pos::from_offsets(
        source,
        filepath,
        snippet.pos.start,
        snippet.pos.start + statement_end,
    );
    let declaration_pos = Pos::from_offsets(
        source,
        filepath,
        snippet.pos.start,
        snippet.pos.start + value_end,
    );
    Ok(Node {
        pos: statement_pos.clone(),
        value: Statement::Declaration(Node {
            pos: declaration_pos,
            value: Declaration {
                keyword: keyword_node,
                identifier,
                datatype,
                value,
            },
        }),
    })
}

fn parse_return_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    let mut index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if !consume_keyword(&snippet.text, &mut index, "return") {
        return Err(snippet_error(
            source,
            filepath,
            snippet.pos.start + index,
            snippet.pos.start + (index + 1).min(snippet.text.len()),
            "expected 'return'",
        ));
    }

    skip_inline_ws(&snippet.text, &mut index);
    let value = if snippet.text.as_bytes().get(index) == Some(&b';') {
        None
    } else {
        let expr_end = find_statement_semicolon(&snippet.text, index).ok_or_else(|| {
            snippet_error(
                source,
                filepath,
                snippet.pos.start + index,
                snippet.pos.end,
                "expected ';' after return value",
            )
        })?;
        let expr_snippet = sub_snippet(source, filepath, snippet, index, expr_end)?;
        index = expr_end;
        Some(parse_expression_snippet(source, &expr_snippet, filepath)?)
    };

    let statement_end = expect_statement_end(source, filepath, snippet, &mut index)?;
    let pos = Pos::from_offsets(
        source,
        filepath,
        snippet.pos.start,
        snippet.pos.start + statement_end,
    );
    Ok(Node {
        pos: pos.clone(),
        value: Statement::FnReturn(Node {
            pos: pos.clone(),
            value: FnReturn { value, pos },
        }),
    })
}

fn parse_label_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    let mut index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if !consume_keyword(&snippet.text, &mut index, "label") {
        return Err(snippet_error(
            source,
            filepath,
            snippet.pos.start + index,
            snippet.pos.start + (index + 1).min(snippet.text.len()),
            "expected 'label'",
        ));
    }

    let name = parse_identifier_node(source, filepath, snippet, &mut index)?;
    let statement_end = expect_statement_end(source, filepath, snippet, &mut index)?;
    let pos = Pos::from_offsets(
        source,
        filepath,
        snippet.pos.start,
        snippet.pos.start + statement_end,
    );
    Ok(Node {
        pos: pos.clone(),
        value: Statement::Label(Node {
            pos,
            value: LabelStatement { name },
        }),
    })
}

fn parse_break_loop_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    let mut index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    let kind = if consume_keyword(&snippet.text, &mut index, "break") {
        BreakLoopType::Break
    } else if consume_keyword(&snippet.text, &mut index, "continue") {
        BreakLoopType::Continue
    } else {
        return Err(snippet_error(
            source,
            filepath,
            snippet.pos.start + index,
            snippet.pos.start + (index + 1).min(snippet.text.len()),
            "expected 'break' or 'continue'",
        ));
    };

    skip_inline_ws(&snippet.text, &mut index);
    let label = if snippet.text.as_bytes().get(index) == Some(&b';') {
        None
    } else {
        Some(
            parse_identifier_node(source, filepath, snippet, &mut index)?
                .value
                .value,
        )
    };

    let statement_end = expect_statement_end(source, filepath, snippet, &mut index)?;
    let pos = Pos::from_offsets(
        source,
        filepath,
        snippet.pos.start,
        snippet.pos.start + statement_end,
    );
    Ok(Node {
        pos: pos.clone(),
        value: Statement::BreakLoop(Node {
            pos,
            value: BreakLoopControl { kind, label },
        }),
    })
}

fn parse_if_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    let mut index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    consume_keyword(&snippet.text, &mut index, "if");
    let cond_start = index;
    let then_start = find_top_level_block_start(&snippet.text, cond_start).ok_or_else(|| {
        snippet_error(
            source,
            filepath,
            snippet.pos.start + cond_start,
            snippet.pos.end,
            "expected block after if condition",
        )
    })?;
    let cond_snippet = sub_snippet_preserve_end(source, filepath, snippet, cond_start, then_start)?;
    let mut condition = parse_expression_snippet(source, &cond_snippet, filepath)?;
    condition.pos = Pos::from_offsets(
        source,
        filepath,
        cond_snippet.pos.start,
        snippet.pos.start + then_start,
    );
    let (then_end, then_block) = parse_code_block_snippet(source, snippet, filepath, then_start)?;

    let mut index = then_end;
    skip_inline_ws(&snippet.text, &mut index);
    let else_block = if consume_keyword(&snippet.text, &mut index, "else") {
        skip_inline_ws(&snippet.text, &mut index);
        if snippet.text[index..].starts_with("if") {
            let child = Box::new(parse_statement_from_offset(
                source, filepath, snippet, index,
            )?);
            Some(Box::new(Node {
                pos: child.pos.clone(),
                value: Block {
                    children: vec![*child],
                },
            }))
        } else {
            let (_, block) = parse_code_block_snippet(source, snippet, filepath, index)?;
            Some(Box::new(block))
        }
    } else {
        None
    };

    let end = snippet.text.len();
    Ok(Node {
        pos: Pos::from_offsets(source, filepath, snippet.pos.start, snippet.pos.start + end),
        value: Statement::If(Node {
            pos: Pos::from_offsets(source, filepath, snippet.pos.start, snippet.pos.start + end),
            value: IfControl {
                condition,
                then_block: Box::new(then_block),
                else_block,
            },
        }),
    })
}

fn parse_while_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    let mut index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    consume_keyword(&snippet.text, &mut index, "while");
    let cond_start = index;
    let body_start = find_top_level_block_start(&snippet.text, cond_start).ok_or_else(|| {
        snippet_error(
            source,
            filepath,
            snippet.pos.start + cond_start,
            snippet.pos.end,
            "expected block after while condition",
        )
    })?;
    let cond_snippet = sub_snippet_preserve_end(source, filepath, snippet, cond_start, body_start)?;
    let mut condition = parse_expression_snippet(source, &cond_snippet, filepath)?;
    condition.pos = Pos::from_offsets(
        source,
        filepath,
        cond_snippet.pos.start,
        snippet.pos.start + body_start,
    );
    let (_, then_block) = parse_code_block_snippet(source, snippet, filepath, body_start)?;
    let end = snippet.text.len();
    Ok(Node {
        pos: Pos::from_offsets(source, filepath, snippet.pos.start, snippet.pos.start + end),
        value: Statement::While(Node {
            pos: Pos::from_offsets(source, filepath, snippet.pos.start, snippet.pos.start + end),
            value: WhileControl {
                condition,
                then_block: Box::new(then_block),
            },
        }),
    })
}

fn parse_loop_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    let mut index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    consume_keyword(&snippet.text, &mut index, "loop");
    let statement = Box::new(parse_statement_from_offset(
        source, filepath, snippet, index,
    )?);
    let end = snippet.text.len();
    Ok(Node {
        pos: Pos::from_offsets(source, filepath, snippet.pos.start, snippet.pos.start + end),
        value: Statement::Loop(Node {
            pos: Pos::from_offsets(source, filepath, snippet.pos.start, snippet.pos.start + end),
            value: LoopControl { statement },
        }),
    })
}

fn parse_atomic_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    let mut index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    let atomic_start = index;
    if snippet.text[index..].starts_with('@') {
        index += 1;
    } else {
        consume_keyword(&snippet.text, &mut index, "atomic");
    }
    skip_inline_ws(&snippet.text, &mut index);
    let mut statement = Box::new(parse_statement_from_offset(
        source, filepath, snippet, index,
    )?);
    let delegated = mark_atomic_wait_delegation(statement.as_mut());
    let end = snippet.text.len();
    let pos = Pos::from_offsets(
        source,
        filepath,
        snippet.pos.start + atomic_start,
        snippet.pos.start + end,
    );
    Ok(Node {
        pos: pos.clone(),
        value: Statement::Atomic(Node {
            pos,
            value: Atomic {
                statement,
                delegated,
            },
        }),
    })
}

fn parse_for_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    let mut index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    consume_keyword(&snippet.text, &mut index, "for");
    let identifier = parse_identifier_node(source, filepath, snippet, &mut index)?;
    skip_inline_ws(&snippet.text, &mut index);
    if !consume_keyword(&snippet.text, &mut index, "in") {
        return Err(snippet_error(
            source,
            filepath,
            snippet.pos.start + index,
            snippet.pos.end,
            "expected 'in' in for statement",
        ));
    }
    let expr_start = index;
    let stmt_start =
        find_statement_start_after_expression(&snippet.text, expr_start).ok_or_else(|| {
            snippet_error(
                source,
                filepath,
                snippet.pos.start + expr_start,
                snippet.pos.end,
                "expected loop body",
            )
        })?;
    let expr_snippet = sub_snippet_preserve_end(source, filepath, snippet, expr_start, stmt_start)?;
    let expression = parse_list_expression_with_chumsky(source, &expr_snippet, filepath)?;
    let statement = Box::new(parse_statement_from_offset(
        source, filepath, snippet, stmt_start,
    )?);
    let end = snippet.text.len();
    Ok(Node {
        pos: Pos::from_offsets(source, filepath, snippet.pos.start, snippet.pos.start + end),
        value: Statement::For(Node {
            pos: Pos::from_offsets(source, filepath, snippet.pos.start, snippet.pos.start + end),
            value: ForControl {
                identifier,
                expression,
                statement,
            },
        }),
    })
}

fn parse_wait_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    let mut index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if consume_keyword(&snippet.text, &mut index, "await") {
        // already consumed
    } else if consume_keyword(&snippet.text, &mut index, "wait") {
        // already consumed
    } else {
        return Err(snippet_error(
            source,
            filepath,
            snippet.pos.start,
            snippet.pos.end,
            "expected wait statement",
        ));
    }

    skip_inline_ws(&snippet.text, &mut index);
    let (block_kind, waiting_cases) = if consume_keyword(&snippet.text, &mut index, "first") {
        skip_inline_ws(&snippet.text, &mut index);
        (
            WaitingBlockKind::First,
            parse_waiting_block_cases(source, snippet, filepath, index)?,
        )
    } else if consume_keyword(&snippet.text, &mut index, "seq") {
        skip_inline_ws(&snippet.text, &mut index);
        (
            WaitingBlockKind::Seq,
            parse_waiting_block_cases(source, snippet, filepath, index)?,
        )
    } else {
        (
            WaitingBlockKind::First,
            vec![parse_waiting_case(source, snippet, filepath, index, snippet.text.len())?.0],
        )
    };

    let pos = Pos::from_offsets(source, filepath, snippet.pos.start, snippet.pos.end);
    Ok(Node {
        pos: pos.clone(),
        value: Statement::Wait(Node {
            pos,
            value: Wait {
                block_kind,
                waiting_cases,
                start_atomic: false,
            },
        }),
    })
}

fn parse_run_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    parse_semicolon_terminated_leaf_statement(
        source,
        snippet,
        filepath,
        "expected ';' after run statement",
        |inner| {
            Ok(Statement::Run(parse_run_call_with_chumsky(
                source, inner, filepath,
            )?))
        },
    )
}

fn parse_semicolon_terminated_leaf_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
    missing_semicolon_message: &'static str,
    build: impl FnOnce(&SyntaxSnippet) -> AlthreadResult<Statement>,
) -> AlthreadResult<Node<Statement>> {
    let expr_end = find_statement_semicolon(&snippet.text, 0).ok_or_else(|| {
        snippet_error(
            source,
            filepath,
            snippet.pos.start,
            snippet.pos.end,
            missing_semicolon_message,
        )
    })?;
    let inner_snippet = sub_snippet(source, filepath, snippet, 0, expr_end)?;
    let statement_end = expr_end + 1;

    Ok(Node {
        pos: Pos::from_offsets(
            source,
            filepath,
            snippet.pos.start,
            snippet.pos.start + statement_end,
        ),
        value: build(&inner_snippet)?,
    })
}

fn parse_send_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    parse_semicolon_terminated_leaf_statement(
        source,
        snippet,
        filepath,
        "expected ';' after send statement",
        |inner| {
            Ok(Statement::Send(parse_send_call_with_chumsky(
                source, inner, filepath,
            )?))
        },
    )
}

fn parse_fn_call_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    parse_semicolon_terminated_leaf_statement(
        source,
        snippet,
        filepath,
        "expected ';' after function call",
        |inner| {
            Ok(Statement::FnCall(parse_fn_call_with_chumsky(
                source, inner, filepath,
            )?))
        },
    )
}

fn parse_assignment_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    let (op_start, op_len, operator) =
        find_assignment_operator(&snippet.text).ok_or_else(|| {
            snippet_error(
                source,
                filepath,
                snippet.pos.start,
                snippet.pos.end,
                "expected assignment operator",
            )
        })?;

    let lhs_snippet = sub_snippet(source, filepath, snippet, 0, op_start)?;
    let mut identifier = parse_object_identifier_with_chumsky(source, &lhs_snippet, filepath)?;
    identifier.pos.end = snippet.pos.start + op_start;
    let value_start = op_start + op_len;
    let value_end = find_statement_semicolon(&snippet.text, value_start).ok_or_else(|| {
        snippet_error(
            source,
            filepath,
            snippet.pos.start + value_start,
            snippet.pos.end,
            "expected ';' after assignment value",
        )
    })?;
    let value_snippet = sub_snippet(source, filepath, snippet, value_start, value_end)?;
    let value = parse_side_effect_expression_snippet(source, &value_snippet, filepath)?;
    let operator_node = Node {
        pos: Pos::from_offsets(
            source,
            filepath,
            snippet.pos.start + op_start,
            snippet.pos.start + op_start + op_len,
        ),
        value: operator,
    };
    let assign_end = value_end;
    let statement_end = assign_end + 1;
    Ok(Node {
        pos: Pos::from_offsets(
            source,
            filepath,
            snippet.pos.start,
            snippet.pos.start + statement_end,
        ),
        value: Statement::Assignment(Node {
            pos: Pos::from_offsets(
                source,
                filepath,
                snippet.pos.start,
                snippet.pos.start + assign_end,
            ),
            value: Assignment::Binary(Node {
                pos: Pos::from_offsets(
                    source,
                    filepath,
                    snippet.pos.start,
                    snippet.pos.start + assign_end,
                ),
                value: BinaryAssignment {
                    identifier,
                    operator: operator_node,
                    value,
                },
            }),
        }),
    })
}

fn parse_channel_declaration_statement(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    parse_semicolon_terminated_leaf_statement(
        source,
        snippet,
        filepath,
        "expected ';' after channel declaration",
        |inner| {
            Ok(Statement::ChannelDeclaration(
                parse_channel_declaration_with_chumsky(source, inner, filepath)?,
            ))
        },
    )
}

fn parse_waiting_block_cases(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
    start: usize,
) -> AlthreadResult<Vec<Node<WaitingBlockCase>>> {
    if snippet.text.as_bytes().get(start) != Some(&b'{') {
        return Err(snippet_error(
            source,
            filepath,
            snippet.pos.start + start,
            snippet.pos.end,
            "expected '{' in wait block",
        ));
    }
    let end = consume_balanced_block(&snippet.text, start, '{', '}').ok_or_else(|| {
        snippet_error(
            source,
            filepath,
            snippet.pos.start + start,
            snippet.pos.end,
            "unterminated wait block",
        )
    })?;
    let mut cases = Vec::new();
    let mut index = start + 1;
    while index < end - 1 {
        skip_inline_ws(&snippet.text, &mut index);
        if index >= end - 1 {
            break;
        }
        let (case, case_end) = parse_waiting_case(source, snippet, filepath, index, end - 1)?;
        cases.push(case);
        index = case_end;
        skip_inline_ws(&snippet.text, &mut index);
    }
    Ok(cases)
}

fn parse_waiting_case(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
    start: usize,
    end: usize,
) -> AlthreadResult<(Node<WaitingBlockCase>, usize)> {
    let (rule_end, separator) =
        find_wait_case_separator(&snippet.text, start, end).ok_or_else(|| {
            snippet_error(
                source,
                filepath,
                snippet.pos.start + start,
                snippet.pos.start + end,
                "expected ';' or '=>' in wait case",
            )
        })?;

    let rule_snippet = sub_snippet_preserve_end(source, filepath, snippet, start, rule_end)?;
    let rule = if starts_with_keyword(&rule_snippet.text, 0, "receive") {
        WaitingBlockCaseRule::Receive(parse_receive_expression(source, &rule_snippet, filepath)?)
    } else {
        let mut expression = parse_expression_snippet(source, &rule_snippet, filepath)?;
        expression.pos = rule_snippet.pos.clone();
        WaitingBlockCaseRule::Expression(expression)
    };

    let (statement, case_end) = if separator == WaitCaseSeparator::Arrow {
        let mut statement_start = rule_end + 2;
        skip_inline_ws(&snippet.text, &mut statement_start);
        let statement_end = find_statement_end_in_range(&snippet.text, statement_start, end)
            .ok_or_else(|| {
                snippet_error(
                    source,
                    filepath,
                    snippet.pos.start + statement_start,
                    snippet.pos.start + end,
                    "expected statement after '=>'",
                )
            })?;
        let statement_snippet =
            sub_snippet(source, filepath, snippet, statement_start, statement_end)?;
        let statement = parse_statement_strict(source, &statement_snippet, filepath)?;
        let mut statement = statement;
        if !matches!(statement.value, Statement::Block(_)) {
            statement.pos = Pos::from_offsets(
                source,
                filepath,
                snippet.pos.start + rule_end + 2,
                snippet.pos.start + statement_end,
            );
        }
        let mut case_end = statement_end;
        if matches!(statement.value, Statement::Block(_)) {
            let mut semicolon_index = case_end;
            skip_inline_ws(&snippet.text, &mut semicolon_index);
            if snippet.text.as_bytes().get(semicolon_index) == Some(&b';') {
                case_end = semicolon_index + 1;
            }
        }
        (Some(statement), case_end)
    } else {
        (None, rule_end + 1)
    };

    Ok((
        Node {
            pos: Pos::from_offsets(
                source,
                filepath,
                snippet.pos.start + start,
                snippet.pos.start + case_end,
            ),
            value: WaitingBlockCase { rule, statement },
        },
        case_end,
    ))
}

fn parse_receive_expression(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<ReceiveStatement>> {
    let mut index = 0;
    skip_inline_ws(&snippet.text, &mut index);
    if !consume_keyword(&snippet.text, &mut index, "receive") {
        return Err(snippet_error(
            source,
            filepath,
            snippet.pos.start,
            snippet.pos.end,
            "expected receive expression",
        ));
    }
    skip_inline_ws(&snippet.text, &mut index);

    let channel = if snippet.text.as_bytes().get(index) == Some(&b'(') {
        String::new()
    } else {
        parse_object_identifier_node(source, filepath, snippet, &mut index)?
            .value
            .parts
            .iter()
            .map(|part| part.value.value.as_str())
            .collect::<Vec<_>>()
            .join(".")
    };

    skip_inline_ws(&snippet.text, &mut index);
    if snippet.text.as_bytes().get(index) != Some(&b'(') {
        return Err(snippet_error(
            source,
            filepath,
            snippet.pos.start + index,
            snippet.pos.end,
            "expected pattern list",
        ));
    }
    let pattern_end = consume_balanced_block(&snippet.text, index, '(', ')').ok_or_else(|| {
        snippet_error(
            source,
            filepath,
            snippet.pos.start + index,
            snippet.pos.end,
            "unterminated pattern list",
        )
    })?;
    let variables = split_top_level_commas(&snippet.text[index + 1..pattern_end - 1])
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    Ok(Node {
        pos: Pos::from_offsets(
            source,
            filepath,
            snippet.pos.start,
            snippet.pos.start + pattern_end,
        ),
        value: ReceiveStatement { channel, variables },
    })
}

fn parse_code_block_snippet(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
    start: usize,
) -> AlthreadResult<(usize, Node<Block>)> {
    let end = consume_balanced_block(&snippet.text, start, '{', '}').ok_or_else(|| {
        snippet_error(
            source,
            filepath,
            snippet.pos.start + start,
            snippet.pos.end,
            "unterminated block",
        )
    })?;
    let block_snippet = SyntaxSnippet::new(
        Pos::from_offsets(
            source,
            filepath,
            snippet.pos.start + start,
            snippet.pos.start + end,
        ),
        snippet.text[start..end].to_string(),
    );
    let (pos, body) = parse_statement_block_with_chumsky(source, &block_snippet, filepath)?;
    Ok((end, build_statement_block(source, &body, pos, filepath)?))
}

fn parse_statement_from_offset(
    source: &str,
    filepath: &str,
    snippet: &SyntaxSnippet,
    start: usize,
) -> AlthreadResult<Node<Statement>> {
    let sub = SyntaxSnippet::new(
        Pos::from_offsets(source, filepath, snippet.pos.start + start, snippet.pos.end),
        snippet.text[start..].to_string(),
    );
    parse_statement_strict(source, &sub, filepath)
}

fn parse_statement_strict(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<Node<Statement>> {
    parse_statement_with_chumsky(source, snippet, filepath)?.ok_or_else(|| {
        AlthreadError::new(
            ErrorType::SyntaxError,
            Some(snippet.pos.clone()),
            "unsupported statement on chumsky path".to_string(),
        )
    })
}

fn mark_atomic_wait_delegation(statement: &mut Node<Statement>) -> bool {
    let mut current = statement;
    loop {
        match &mut current.value {
            Statement::Wait(wait) => {
                wait.value.start_atomic = true;
                return true;
            }
            Statement::Block(block) => {
                if let Some(child) = block.value.children.first_mut() {
                    current = child;
                } else {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WaitCaseSeparator {
    Arrow,
    Semicolon,
}

fn parse_identifier_node(
    source: &str,
    filepath: &str,
    snippet: &SyntaxSnippet,
    index: &mut usize,
) -> AlthreadResult<Node<Identifier>> {
    skip_inline_ws(&snippet.text, index);
    let start = *index;
    *index = consume_identifier(&snippet.text, *index).ok_or_else(|| {
        snippet_error(
            source,
            filepath,
            snippet.pos.start + start,
            snippet.pos.start + (start + 1).min(snippet.text.len()),
            "expected identifier",
        )
    })?;

    Ok(Node {
        pos: Pos::from_offsets(
            source,
            filepath,
            snippet.pos.start + start,
            snippet.pos.start + *index,
        ),
        value: Identifier {
            value: snippet.text[start..*index].to_string(),
        },
    })
}

fn parse_object_identifier_node(
    source: &str,
    filepath: &str,
    snippet: &SyntaxSnippet,
    index: &mut usize,
) -> AlthreadResult<Node<ObjectIdentifier>> {
    skip_inline_ws(&snippet.text, index);
    let start = *index;
    let mut parts = vec![parse_identifier_node(source, filepath, snippet, index)?];
    loop {
        if snippet.text.as_bytes().get(*index) != Some(&b'.') {
            break;
        }
        *index += 1;
        parts.push(parse_identifier_node(source, filepath, snippet, index)?);
    }
    let end = parts
        .last()
        .map(|part| part.pos.end)
        .unwrap_or(snippet.pos.start + start);

    Ok(Node {
        pos: Pos::from_offsets(source, filepath, snippet.pos.start + start, end),
        value: ObjectIdentifier { parts },
    })
}

fn expect_statement_end(
    source: &str,
    filepath: &str,
    snippet: &SyntaxSnippet,
    index: &mut usize,
) -> AlthreadResult<usize> {
    skip_inline_ws(&snippet.text, index);
    if snippet.text.as_bytes().get(*index) != Some(&b';') {
        return Err(snippet_error(
            source,
            filepath,
            snippet.pos.start + *index,
            snippet.pos.start + (*index + 1).min(snippet.text.len()),
            "expected ';'",
        ));
    }
    *index += 1;
    skip_inline_ws(&snippet.text, index);
    if *index != snippet.text.len() {
        return Err(snippet_error(
            source,
            filepath,
            snippet.pos.start + *index,
            snippet.pos.end,
            "unexpected trailing input in statement",
        ));
    }
    Ok(*index)
}

fn find_statement_semicolon(text: &str, start: usize) -> Option<usize> {
    find_top_level_char(text, start, &[';'])
}

fn find_wait_case_separator(
    text: &str,
    start: usize,
    limit: usize,
) -> Option<(usize, WaitCaseSeparator)> {
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let chars: Vec<(usize, char)> = text[start..limit].char_indices().collect();
    let mut idx = 0usize;

    while idx < chars.len() {
        let (offset, ch) = chars[idx];
        let byte_idx = start + offset;
        if in_string {
            if escaped {
                escaped = false;
            } else {
                match ch {
                    '\\' => escaped = true,
                    '"' => in_string = false,
                    _ => {}
                }
            }
            idx += 1;
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            ';' if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 => {
                return Some((byte_idx, WaitCaseSeparator::Semicolon));
            }
            '=' if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 => {
                if text[byte_idx..limit].starts_with("=>") {
                    return Some((byte_idx, WaitCaseSeparator::Arrow));
                }
            }
            _ => {}
        }
        idx += 1;
    }

    None
}

fn find_statement_end_in_range(text: &str, start: usize, limit: usize) -> Option<usize> {
    let sub = &text[..limit];
    let end = if sub.as_bytes().get(start) == Some(&b'{') {
        consume_balanced_block(sub, start, '{', '}')?
    } else if starts_with_keyword(sub, start, "if") {
        consume_if_statement_end(sub, start)?
    } else if starts_with_keyword(sub, start, "while") {
        consume_while_statement_end(sub, start)?
    } else if starts_with_keyword(sub, start, "loop") {
        let mut idx = start + "loop".len();
        skip_inline_ws(sub, &mut idx);
        find_statement_end_in_range(sub, idx, limit)?
    } else if starts_with_keyword(sub, start, "for") {
        consume_for_statement_end(sub, start, limit)?
    } else if starts_with_keyword(sub, start, "atomic") || sub[start..].starts_with('@') {
        let mut idx = start;
        if sub[idx..].starts_with('@') {
            idx += 1;
        } else {
            idx += "atomic".len();
        }
        skip_inline_ws(sub, &mut idx);
        find_statement_end_in_range(sub, idx, limit)?
    } else {
        find_statement_semicolon(sub, start)? + 1
    };
    Some(end)
}

fn consume_if_statement_end(text: &str, start: usize) -> Option<usize> {
    let mut index = start;
    if !consume_keyword(text, &mut index, "if") {
        return None;
    }
    skip_inline_ws(text, &mut index);
    let then_start = find_top_level_block_start(text, index)?;
    let then_end = consume_balanced_block(text, then_start, '{', '}')?;
    index = then_end;
    skip_inline_ws(text, &mut index);
    if consume_keyword(text, &mut index, "else") {
        skip_inline_ws(text, &mut index);
        if starts_with_keyword(text, index, "if") {
            consume_if_statement_end(text, index)
        } else {
            consume_balanced_block(text, index, '{', '}')
        }
    } else {
        Some(index)
    }
}

fn consume_while_statement_end(text: &str, start: usize) -> Option<usize> {
    let mut index = start;
    if !consume_keyword(text, &mut index, "while") {
        return None;
    }
    skip_inline_ws(text, &mut index);
    let body_start = find_top_level_block_start(text, index)?;
    consume_balanced_block(text, body_start, '{', '}')
}

fn consume_for_statement_end(text: &str, start: usize, limit: usize) -> Option<usize> {
    let mut index = start;
    if !consume_keyword(text, &mut index, "for") {
        return None;
    }
    index = consume_identifier(text, index)?;
    skip_inline_ws(text, &mut index);
    if !consume_keyword(text, &mut index, "in") {
        return None;
    }
    skip_inline_ws(text, &mut index);
    let stmt_start = find_statement_start_after_expression_in_range(text, index, limit)?;
    find_statement_end_in_range(text, stmt_start, limit)
}

fn find_statement_start_after_expression_in_range(
    text: &str,
    start: usize,
    limit: usize,
) -> Option<usize> {
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut seen_content = false;

    for (offset, ch) in text[start..limit].char_indices() {
        let idx = start + offset;
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => {
                if seen_content && depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 {
                    return Some(idx);
                }
                depth_brace += 1;
            }
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 => {
                if ch.is_whitespace() {
                    if seen_content {
                        let mut next = idx + ch.len_utf8();
                        while next < limit
                            && text
                                .as_bytes()
                                .get(next)
                                .is_some_and(|byte| byte.is_ascii_whitespace())
                        {
                            next += 1;
                        }
                        return (next < limit).then_some(next);
                    }
                } else {
                    seen_content = true;
                }
            }
            _ => {}
        }
    }

    None
}

fn split_top_level_commas(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            ',' if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 => {
                parts.push(&text[start..idx]);
                start = idx + 1;
            }
            _ => {}
        }
    }

    if start <= text.len() {
        parts.push(&text[start..]);
    }
    parts
}

fn starts_with_keyword(text: &str, index: usize, keyword: &str) -> bool {
    if index > text.len() || !text[index..].starts_with(keyword) {
        return false;
    }
    let end = index + keyword.len();
    end >= text.len() || !is_ident_char(text.as_bytes()[end])
}

fn find_top_level_block_start(text: &str, start: usize) -> Option<usize> {
    find_top_level_char(text, start, &['{'])
}

fn find_statement_start_after_expression(text: &str, start: usize) -> Option<usize> {
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut seen_content = false;

    for (offset, ch) in text[start..].char_indices() {
        let idx = start + offset;
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => {
                if seen_content && depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 {
                    return Some(idx);
                }
                depth_brace += 1;
            }
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 => {
                if ch.is_whitespace() {
                    if seen_content {
                        let mut next = idx + ch.len_utf8();
                        while next < text.len()
                            && text
                                .as_bytes()
                                .get(next)
                                .is_some_and(|byte| byte.is_ascii_whitespace())
                        {
                            next += 1;
                        }
                        return (next < text.len()).then_some(next);
                    }
                } else {
                    seen_content = true;
                }
            }
            _ => {}
        }
    }

    None
}

fn consume_balanced_block(text: &str, start: usize, open: char, close: char) -> Option<usize> {
    if text[start..].chars().next()? != open {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in text[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            c if c == open => depth += 1,
            c if c == close => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(start + offset + ch.len_utf8());
                }
            }
            _ => {}
        }
    }

    None
}

fn looks_like_call_statement(text: &str) -> bool {
    let Some(semicolon) = find_statement_semicolon(text, 0) else {
        return false;
    };
    let body = text[..semicolon].trim();
    if body.is_empty() || has_top_level_assignment(text) {
        return false;
    }

    let bytes = body.as_bytes();
    let mut index = 0usize;
    index = match consume_identifier(body, index) {
        Some(index) => index,
        None => return false,
    };

    loop {
        if bytes.get(index) != Some(&b'.') {
            break;
        }
        index += 1;
        index = match consume_identifier(body, index) {
            Some(index) => index,
            None => return false,
        };
    }

    bytes.get(index) == Some(&b'(')
}

fn has_top_level_assignment(text: &str) -> bool {
    find_assignment_operator(text).is_some()
}

fn find_assignment_operator(text: &str) -> Option<(usize, usize, BinaryAssignmentOperator)> {
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut idx = 0usize;

    while idx < chars.len() {
        let (byte_idx, ch) = chars[idx];
        if in_string {
            if escaped {
                escaped = false;
            } else {
                match ch {
                    '\\' => escaped = true,
                    '"' => in_string = false,
                    _ => {}
                }
            }
            idx += 1;
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 => {
                let rest = &text[byte_idx..];
                let found = if rest.starts_with("+=") {
                    Some((2, BinaryAssignmentOperator::AddAssign))
                } else if rest.starts_with("-=") {
                    Some((2, BinaryAssignmentOperator::SubtractAssign))
                } else if rest.starts_with("*=") {
                    Some((2, BinaryAssignmentOperator::MultiplyAssign))
                } else if rest.starts_with("/=") {
                    Some((2, BinaryAssignmentOperator::DivideAssign))
                } else if rest.starts_with("%=") {
                    Some((2, BinaryAssignmentOperator::ModuloAssign))
                } else if rest.starts_with("|=") {
                    Some((2, BinaryAssignmentOperator::OrAssign))
                } else if rest.starts_with('=') {
                    Some((1, BinaryAssignmentOperator::Assign))
                } else {
                    None
                };

                if let Some((len, op)) = found {
                    let prev = if byte_idx == 0 {
                        None
                    } else {
                        text[..byte_idx].chars().next_back()
                    };
                    let next = text[byte_idx + len..].chars().next();
                    if matches!(prev, Some('=') | Some('!') | Some('<') | Some('>')) {
                        idx += 1;
                        continue;
                    }
                    if matches!(next, Some('>') | Some('=')) {
                        idx += 1;
                        continue;
                    }
                    return Some((byte_idx, len, op));
                }
            }
            _ => {}
        }
        idx += 1;
    }

    None
}

fn find_top_level_char(text: &str, start: usize, needles: &[char]) -> Option<usize> {
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in text[start..].char_indices() {
        let idx = start + offset;
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 && needles.contains(&ch) {
            return Some(idx);
        }

        match ch {
            '"' => in_string = true,
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }
    }

    None
}

fn sub_snippet(
    source: &str,
    filepath: &str,
    snippet: &SyntaxSnippet,
    start: usize,
    end: usize,
) -> AlthreadResult<SyntaxSnippet> {
    let trimmed_start = start
        + snippet.text[start..end]
            .chars()
            .take_while(|ch| ch.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();
    let trimmed_end = end
        - snippet.text[start..end]
            .chars()
            .rev()
            .take_while(|ch| ch.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();

    Ok(SyntaxSnippet::new(
        Pos::from_offsets(
            source,
            filepath,
            snippet.pos.start + trimmed_start,
            snippet.pos.start + trimmed_end,
        ),
        snippet.text[trimmed_start..trimmed_end].to_string(),
    ))
}

fn sub_snippet_preserve_end(
    source: &str,
    filepath: &str,
    snippet: &SyntaxSnippet,
    start: usize,
    end: usize,
) -> AlthreadResult<SyntaxSnippet> {
    let trimmed_start = start
        + snippet.text[start..end]
            .chars()
            .take_while(|ch| ch.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();

    Ok(SyntaxSnippet::new(
        Pos::from_offsets(
            source,
            filepath,
            snippet.pos.start + trimmed_start,
            snippet.pos.start + end,
        ),
        snippet.text[trimmed_start..end].to_string(),
    ))
}

fn skip_inline_ws(text: &str, index: &mut usize) {
    while *index < text.len() && text.as_bytes()[*index].is_ascii_whitespace() {
        *index += 1;
    }
}

fn consume_keyword(text: &str, index: &mut usize, keyword: &str) -> bool {
    if *index > text.len() || !text[*index..].starts_with(keyword) {
        return false;
    }
    let end = *index + keyword.len();
    if end < text.len() && is_ident_char(text.as_bytes()[end]) {
        return false;
    }
    *index = end;
    true
}

fn consume_identifier(text: &str, mut index: usize) -> Option<usize> {
    if index >= text.len() || !text.as_bytes()[index].is_ascii_alphabetic() {
        return None;
    }
    index += 1;
    while index < text.len() && is_ident_char(text.as_bytes()[index]) {
        index += 1;
    }
    Some(index)
}

fn is_ident_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn snippet_error(
    source: &str,
    filepath: &str,
    start: usize,
    end: usize,
    message: &str,
) -> AlthreadError {
    AlthreadError::new(
        ErrorType::SyntaxError,
        Some(Pos::from_offsets(source, filepath, start, end)),
        message.to_string(),
    )
}

impl fmt::Display for Ast {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.ast_fmt(f, &Prefix::new())
    }
}

impl AstDisplay for Ast {
    fn ast_fmt(&self, f: &mut Formatter, prefix: &Prefix) -> fmt::Result {
        if let Some(import_block) = &self.import_block {
            import_block.ast_fmt(f, prefix)?;
            writeln!(f, "")?;
        }

        if let Some(global_node) = &self.global_block {
            writeln!(f, "{}shared", prefix)?;
            global_node.ast_fmt(f, &prefix.add_branch())?;
        }

        writeln!(f, "")?;

        for (condition_name, condition_node) in &self.condition_blocks {
            writeln!(f, "{}{}", prefix, condition_name)?;
            condition_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
            for check_block in &self.check_blocks {
                writeln!(f, "{}check", prefix)?;
                for form in &check_block.value.formulas {
                    writeln!(f, "{}{}", prefix.add_branch(), form)?;
                }
            }
        }

        for (process_name, (_args, process_node, is_private)) in &self.process_blocks {
            let process_name = if *is_private {
                format!("@private {}", process_name)
            } else {
                process_name.clone()
            };
            writeln!(f, "{}{}", prefix, process_name)?;
            process_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
        }

        for (function_name, (_args, return_type, function_node, is_private)) in
            &self.function_blocks
        {
            writeln!(f, "{}", if *is_private { "@private " } else { "" })?;
            writeln!(f, "{}{} -> {}", prefix, function_name, return_type)?;
            function_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
        }

        Ok(())
    }
}
