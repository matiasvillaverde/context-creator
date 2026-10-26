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
    Ok(extract_declarations(path, content)?
        .into_iter()
        .map(|declaration| declaration.symbol)
        .collect())
}

#[derive(Debug, Clone)]
pub(crate) struct SourceDeclaration {
    pub symbol: CodeSymbol,
    pub qualified_name: String,
    pub start_byte: usize,
    pub start_line: usize,
    pub end_byte: usize,
    pub end_line: usize,
    pub signature: String,
}

pub(crate) fn extract_declarations(
    path: &Path,
    content: &str,
) -> Result<Vec<SourceDeclaration>, ContextCreatorError> {
    Ok(extract_declarations_with_diagnostics(path, content)?.0)
}

pub(crate) fn extract_declarations_with_diagnostics(
    path: &Path,
    content: &str,
) -> Result<(Vec<SourceDeclaration>, bool), ContextCreatorError> {
    let Some(language) = super::grammar_for_source(path, content) else {
        return Ok((Vec::new(), false));
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
                let declaration = node
                    .child_by_field_name("value")
                    .filter(|value| {
                        matches!(value.kind(), "arrow_function" | "function_expression")
                    })
                    .unwrap_or(node);
                let mut children = declaration.walk();
                let body = declaration.child_by_field_name("body").or_else(|| {
                    declaration.named_children(&mut children).find(|child| {
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
                    let symbol = CodeSymbol {
                        name: content[name.byte_range()].to_string(),
                        kind: kind.into(),
                        line: node.start_position().row + 1,
                        signature,
                    };
                    let mut envelope = node;
                    if node.kind() == "variable_declarator" {
                        if let Some(parent) = node.parent() {
                            envelope = parent;
                        }
                    }
                    if let Some(parent) = envelope.parent() {
                        if matches!(parent.kind(), "decorated_definition" | "export_statement") {
                            envelope = parent;
                        }
                    }
                    let mut start_byte = envelope.start_byte();
                    let mut start_line = envelope.start_position().row + 1;
                    let mut sibling = envelope.prev_named_sibling();
                    while let Some(attribute) =
                        sibling.filter(|node| node.kind() == "attribute_item")
                    {
                        start_byte = attribute.start_byte();
                        start_line = attribute.start_position().row + 1;
                        sibling = attribute.prev_named_sibling();
                    }
                    let mut names = vec![symbol.name.clone()];
                    let mut parent = node.parent();
                    while let Some(ancestor) = parent {
                        if let Some(name) = declaration_name(ancestor) {
                            let name = &content[name.byte_range()];
                            if names.last().is_none_or(|previous| previous != name) {
                                names.push(name.into());
                            }
                        }
                        parent = ancestor.parent();
                    }
                    names.reverse();
                    symbols.push(SourceDeclaration {
                        qualified_name: names.join("::"),
                        start_byte,
                        start_line,
                        end_byte: envelope.end_byte(),
                        end_line: envelope.end_position().row
                            + usize::from(envelope.end_position().column > 0),
                        signature: content[start_byte..end].trim().into(),
                        symbol,
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
    Ok((symbols, tree.root_node().has_error()))
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
