//! Compact declarations for repository maps and progressive exploration.

use crate::core::semantic::parsing::ParserExt;
use crate::utils::error::ContextCreatorError;
use serde::Serialize;
use std::path::Path;
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, Serialize)]
pub struct CodeSymbol {
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub signature: String,
}

pub fn extract_symbols(path: &Path, content: &str) -> Result<Vec<CodeSymbol>, ContextCreatorError> {
    let Some(language) = super::grammar_for_source(path, content) else {
        return Ok(Vec::new());
    };
    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .map_err(|error| ContextCreatorError::ParseError(error.to_string()))?;
    let tree = parser
        .parse_bounded(content, None)
        .ok_or_else(|| ContextCreatorError::ParseError("Declaration parsing timed out".into()))?;
    let mut symbols = Vec::new();
    let mut cursor = tree.walk();
    'walk: loop {
        let node = cursor.node();
        let kind = match node.kind() {
            "function_item"
            | "function_definition"
            | "function_declaration"
            | "method_definition"
            | "method_declaration"
            | "method_signature"
            | "constructor_declaration"
            | "method"
            | "singleton_method" => Some("function"),
            "type_definition"
            | "class"
            | "struct_item"
            | "enum_item"
            | "trait_item"
            | "type_item"
            | "class_definition"
            | "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "type_alias_declaration"
            | "type_spec"
            | "type_alias"
            | "protocol_declaration"
            | "typealias_declaration"
            | "struct_specifier"
            | "class_specifier"
            | "enum_specifier"
            | "interface_definition"
            | "module" => Some("type"),
            "variable_declarator"
                if node.child_by_field_name("value").is_some_and(|value| {
                    matches!(value.kind(), "arrow_function" | "function_expression")
                }) =>
            {
                Some("function")
            }
            _ => None,
        };
        if let Some(kind) = kind {
            if let Some(name) = declaration_name(node) {
                let mut children = node.walk();
                let body = node.child_by_field_name("body").or_else(|| {
                    node.named_children(&mut children).find(|child| {
                        matches!(
                            child.kind(),
                            "function_body"
                                | "class_body"
                                | "enum_class_body"
                                | "field_declaration_list"
                                | "declaration_list"
                                | "block"
                                | "compound_statement"
                                | "statement_block"
                                | "object_type"
                                | "interface_body"
                        )
                    })
                });
                let mut end = body.map(|body| body.start_byte()).unwrap_or_else(|| {
                    let text = &content[node.byte_range()];
                    node.start_byte() + text.find('\n').unwrap_or(text.len())
                });
                if matches!(
                    path.extension().and_then(|extension| extension.to_str()),
                    Some("py" | "pyi" | "pyw")
                ) {
                    // Python's block starts after leading comment-only lines;
                    // the declaration's own colon is the true header boundary.
                    let mut children = node.walk();
                    if let Some(colon) = node
                        .children(&mut children)
                        .find(|child| child.kind() == ":")
                    {
                        end = colon.end_byte();
                    };
                }
                let signature = content[node.start_byte()..end]
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                if !signature.is_empty() {
                    symbols.push(CodeSymbol {
                        name: content[name.byte_range()].to_string(),
                        kind: kind.into(),
                        line: node.start_position().row + 1,
                        signature,
                    });
                }
            }
        }
        if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                break 'walk;
            }
        }
    }
    Ok(symbols)
}

fn declaration_name(node: Node<'_>) -> Option<Node<'_>> {
    if let Some(name) = node.child_by_field_name("name") {
        return Some(name);
    }
    let mut current = node.child_by_field_name("declarator")?;
    for _ in 0..16 {
        if matches!(
            current.kind(),
            "identifier" | "field_identifier" | "type_identifier"
        ) {
            return Some(current);
        }
        current = current
            .child_by_field_name("declarator")
            .or_else(|| current.child_by_field_name("name"))?;
    }
    None
}
