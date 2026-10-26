use super::{ancestors, child, children, text, Profile};
use crate::core::semantic::analyzer::Import;
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(super) fn html_attributes(tag: Node<'_>, source: &str) -> BTreeMap<String, String> {
    children(tag)
        .into_iter()
        .filter(|node| node.kind() == "attribute")
        .filter_map(|attribute| {
            let values = children(attribute);
            Some((
                text(*values.first()?, source).to_ascii_lowercase(),
                static_value(*values.get(1)?, source, true)?,
            ))
        })
        .collect()
}
fn static_value(node: Node<'_>, source: &str, bare: bool) -> Option<String> {
    let raw = text(node, source).trim();
    if raw.starts_with(['\'', '"']) {
        let quote = raw.chars().next()?;
        let value = raw.strip_prefix(quote)?.strip_suffix(quote)?;
        if value.contains(['\\', '`']) || quote == '"' && value.contains('$') {
            return None;
        }
        Some(value.to_owned())
    } else if bare
        && !raw.is_empty()
        && !raw
            .chars()
            .any(|ch| ch.is_whitespace() || matches!(ch, '$' | '`' | '{' | '}' | '(' | ')'))
    {
        Some(raw.to_owned())
    } else {
        None
    }
}
fn relative_file(path: String) -> String {
    if !path.starts_with(['.', '/']) && !path.contains(':') {
        format!("./{path}")
    } else {
        path
    }
}

fn powershell_value(node: Node<'_>, source: &str) -> Option<String> {
    let raw = text(node, source).trim();
    if raw.contains('`') || !raw.starts_with("'") && raw.contains('$') {
        return None;
    }
    let value = raw.trim_matches(['\'', '"']);
    (!value.is_empty() && !value.contains(['\n', '\r'])).then(|| value.replace('\\', "/"))
}

fn first_argument(node: Node<'_>) -> Option<Node<'_>> {
    let arguments = node
        .child_by_field_name("arguments")
        .or_else(|| node.child_by_field_name("args"))
        .or_else(|| child(node, "arguments"))
        .or_else(|| child(node, "argument_list"))
        .or_else(|| child(node, "function_arguments"))?;
    let value = children(arguments).into_iter().next()?;
    if matches!(value.kind(), "argument" | "expression" | "argument_list") {
        children(value).into_iter().next().or(Some(value))
    } else {
        Some(value)
    }
}
pub(super) fn add_import(
    imports: &mut Vec<Import>,
    module: Option<String>,
    relative: bool,
    node: Node<'_>,
) {
    let item = Import {
        module: module.unwrap_or_else(|| "<computed import>".into()),
        items: vec![],
        is_relative: relative,
        line: node.start_position().row + 1,
    };
    if !imports.contains(&item) {
        imports.push(item);
    }
}

impl Profile {
    pub(super) fn imports(self, node: Node<'_>, source: &str, imports: &mut Vec<Import>) {
        if self == Self::Clojure && ancestors(node).any(|node| node.kind() == "dis_expr") {
            return;
        }
        match self {
            Self::Dart
                if matches!(
                    node.kind(),
                    "import_specification" | "library_export" | "part_directive"
                ) =>
            {
                let value = node.child_by_field_name("uri").or_else(|| {
                    children(node)
                        .into_iter()
                        .find(|child| matches!(child.kind(), "uri" | "string_literal"))
                });
                let value = value.and_then(|value| static_value(value, source, false));
                let relative = value.as_ref().is_some_and(|value| !value.contains(':'));
                add_import(imports, value, relative, node);
            }
            Self::Bash if node.kind() == "command" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|name| text(name, source));
                if matches!(name, Some("source" | ".")) {
                    add_import(
                        imports,
                        node.child_by_field_name("argument")
                            .and_then(|value| static_value(value, source, true)),
                        true,
                        node,
                    );
                }
            }
            Self::Lua | Self::R | Self::Julia
                if matches!(node.kind(), "function_call" | "call" | "call_expression") =>
            {
                let name = node
                    .child_by_field_name("name")
                    .or_else(|| node.child_by_field_name("function"))
                    .or_else(|| children(node).into_iter().next())
                    .map(|name| text(name, source));
                if matches!(
                    (self, name),
                    (Self::Lua, Some("require" | "dofile" | "loadfile"))
                        | (
                            Self::R,
                            Some("source" | "sys.source" | "library" | "require")
                        )
                        | (Self::Julia, Some("include" | "include_dependency"))
                ) {
                    let relative = !matches!(
                        (self, name),
                        (Self::Lua, Some("require")) | (Self::R, Some("library" | "require"))
                    );
                    let value = first_argument(node)
                        .and_then(|value| static_value(value, source, self == Self::R && !relative))
                        .map(|value| {
                            if relative {
                                relative_file(value)
                            } else {
                                value
                            }
                        });
                    add_import(imports, value, relative, node);
                }
            }
            Self::Scala if node.kind() == "import_declaration" => {
                let value = node
                    .child_by_field_name("path")
                    .map(|path| text(path, source).trim_end_matches(".*").to_owned());
                add_import(imports, value, false, node);
            }
            Self::Haskell if node.kind() == "import" => {
                add_import(
                    imports,
                    node.child_by_field_name("module")
                        .map(|module| text(module, source).into()),
                    false,
                    node,
                );
            }
            Self::Julia if matches!(node.kind(), "using_statement" | "import_statement") => {
                for value in children(node)
                    .into_iter()
                    .filter(|node| matches!(node.kind(), "import_path" | "selected_import"))
                {
                    let value = children(value).into_iter().next().unwrap_or(value);
                    add_import(imports, Some(text(value, source).into()), false, node);
                }
            }
            Self::Elixir if node.kind() == "call" => {
                let target = node
                    .child_by_field_name("target")
                    .map(|value| text(value, source));
                if matches!(target, Some("Code.require_file" | "Code.compile_file")) {
                    add_import(
                        imports,
                        first_argument(node).and_then(|value| static_value(value, source, false)),
                        true,
                        node,
                    );
                } else if matches!(target, Some("alias" | "import" | "require" | "use")) {
                    let value =
                        child(node, "arguments").and_then(|args| children(args).into_iter().next());
                    add_import(
                        imports,
                        value
                            .filter(|value| value.kind() == "alias")
                            .map(|value| text(value, source).into()),
                        false,
                        node,
                    );
                }
            }
            Self::Clojure if node.kind() == "list_lit" => {
                let values = children(node);
                let head = values.first().map(|value| text(*value, source));
                if matches!(head, Some(":require" | ":require-macros")) {
                    for value in values.into_iter().skip(1) {
                        let name = if value.kind() == "vec_lit" {
                            children(value).into_iter().next()
                        } else {
                            Some(value)
                        };
                        add_import(
                            imports,
                            name.map(|value| text(value, source).into()),
                            false,
                            node,
                        );
                    }
                } else if matches!(head, Some("load-file")) {
                    add_import(
                        imports,
                        values
                            .get(1)
                            .and_then(|value| static_value(*value, source, false)),
                        true,
                        node,
                    );
                }
            }
            Self::PowerShell if node.kind() == "command" => {
                let name = node
                    .child_by_field_name("command_name")
                    .map(|name| text(name, source).to_owned());
                let dot = child(node, "command_invokation_operator")
                    .is_some_and(|node| text(node, source) == ".");
                if dot {
                    let value = node
                        .child_by_field_name("command_name")
                        .and_then(|value| powershell_value(value, source));
                    add_import(imports, value, true, node);
                } else if name
                    .as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case("import-module"))
                {
                    let value = node
                        .child_by_field_name("command_elements")
                        .and_then(|args| {
                            children(args).into_iter().find(|value| {
                                value.kind() != "command_parameter"
                                    && value.kind() != "command_argument_sep"
                            })
                        });
                    add_import(
                        imports,
                        value.and_then(|value| powershell_value(value, source)),
                        true,
                        node,
                    );
                }
            }
            Self::Perl if matches!(node.kind(), "use_statement" | "require_expression") => {
                let value = node
                    .child_by_field_name("module")
                    .or_else(|| children(node).into_iter().next());
                let value = value.and_then(|value| static_value(value, source, true));
                let relative = value
                    .as_ref()
                    .is_some_and(|value| value.contains('/') || value.ends_with(".pm"));
                add_import(imports, value, relative, node);
            }
            Self::Hcl if node.kind() == "block" => {
                let kind = children(node)
                    .into_iter()
                    .next()
                    .map(|node| text(node, source));
                if matches!(kind, Some("module" | "include")) {
                    if let Some(body) = child(node, "body") {
                        for attribute in children(body)
                            .into_iter()
                            .filter(|child| child.kind() == "attribute")
                        {
                            let values = children(attribute);
                            if values.first().is_some_and(|name| {
                                matches!(text(*name, source), "source" | "path")
                            }) {
                                let value = values
                                    .get(1)
                                    .and_then(|expression| children(*expression).into_iter().next())
                                    .and_then(|value| static_value(value, source, false));
                                let relative =
                                    value.as_ref().is_some_and(|value| value.starts_with('.'));
                                add_import(imports, value, relative, attribute);
                            }
                        }
                    }
                }
            }
            Self::Hcl if node.kind() == "function_call" => {
                let name = child(node, "identifier").map(|name| text(name, source));
                if matches!(name, Some("file" | "templatefile")) {
                    add_import(
                        imports,
                        first_argument(node).and_then(|value| static_value(value, source, false)),
                        true,
                        node,
                    );
                }
            }
            Self::Html
                if matches!(
                    node.kind(),
                    "script_element" | "element" | "self_closing_tag"
                ) =>
            {
                if let Some(tag) = if node.kind() == "self_closing_tag" {
                    Some(node)
                } else {
                    child(node, "start_tag")
                } {
                    let name =
                        child(tag, "tag_name").map(|name| text(name, source).to_ascii_lowercase());
                    let wanted = match name.as_deref() {
                        Some("script") => "src",
                        Some("link") => "href",
                        _ => return,
                    };
                    if wanted == "href"
                        && !html_attributes(tag, source).get("rel").is_some_and(|rel| {
                            rel.split_whitespace().any(|rel| {
                                rel.eq_ignore_ascii_case("stylesheet")
                                    || rel.eq_ignore_ascii_case("modulepreload")
                            })
                        })
                    {
                        return;
                    }
                    for attribute in children(tag)
                        .into_iter()
                        .filter(|child| child.kind() == "attribute")
                    {
                        let values = children(attribute);
                        if values
                            .first()
                            .is_some_and(|name| text(*name, source).eq_ignore_ascii_case(wanted))
                        {
                            let value = values
                                .get(1)
                                .and_then(|value| static_value(*value, source, true));
                            let relative = value.as_ref().is_some_and(|value| {
                                !value.contains("://") && !value.starts_with("//")
                            });
                            add_import(imports, value, relative, attribute);
                        }
                    }
                }
            }
            Self::Css if node.kind() == "import_statement" => {
                let value = children(node).into_iter().next().and_then(|value| {
                    let value = if value.kind() == "call_expression" {
                        first_argument(value)?
                    } else {
                        value
                    };
                    static_value(value, source, false)
                });
                let relative = value.as_ref().is_some_and(|value| !value.contains("://"));
                add_import(imports, value, relative, node);
            }
            Self::Sql
                if node.kind() == "object_reference"
                    && node.parent().is_some_and(|parent| {
                        matches!(
                            parent.kind(),
                            "relation"
                                | "from"
                                | "join"
                                | "references"
                                | "insert"
                                | "update"
                                | "delete"
                        )
                    }) =>
            {
                add_import(
                    imports,
                    Some(format!("sql:{}", text(node, source))),
                    false,
                    node,
                );
            }
            _ => {}
        }
    }
}
