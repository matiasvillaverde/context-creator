//! Zig extraction uses syntax, never executes comptime code or build scripts.

use super::super::{
    analyzer::{
        AnalysisResult, FunctionCall, FunctionDefinition, Import, LanguageAnalyzer,
        SemanticContext, SemanticResult, TypeReference,
    },
    parsing::ParserExt,
    path_validator::{validate_import_path, validate_module_name},
    resolver::{ModuleResolver, ResolvedPath},
    symbols::zig_container,
};
use crate::utils::error::ContextCreatorError;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};
use tree_sitter::{Node, Parser};

#[derive(Default)]
pub struct ZigAnalyzer;

impl LanguageAnalyzer for ZigAnalyzer {
    fn language_name(&self) -> &'static str {
        "Zig"
    }

    fn analyze_file(
        &self,
        path: &Path,
        content: &str,
        context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        self.analyze_requested(path, content, context, true, true, true, true)
    }

    fn analyze_requested(
        &self,
        path: &Path,
        content: &str,
        context: &SemanticContext,
        include_imports: bool,
        include_calls: bool,
        include_types: bool,
        include_functions: bool,
    ) -> SemanticResult<AnalysisResult> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_zig::LANGUAGE.into())
            .map_err(|error| ContextCreatorError::ParseError(error.to_string()))?;
        let tree = parser
            .parse_bounded(content, None)
            .ok_or_else(|| ContextCreatorError::ParseError("Zig parsing timed out".into()))?;
        let mut result = AnalysisResult::default();
        let mut local_types = BTreeSet::new();
        if tree.root_node().has_error() {
            result
                .errors
                .push("Zig source contains syntax errors".into());
        }
        let mut cursor = tree.walk();
        'walk: loop {
            let node = cursor.node();
            if include_types && zig_container(node).is_some() {
                let mut children = node.walk();
                let name = node
                    .named_children(&mut children)
                    .find(|child| child.kind() == "identifier");
                if let Some(name) = name {
                    local_types.insert(text(name, content).to_owned());
                }
            }
            match node.kind() {
                "builtin_function" if include_imports || include_types => {
                    let mut children = node.walk();
                    let mut named = node.named_children(&mut children);
                    let builtin = named.next().map(|name| text(name, content));
                    if matches!(builtin, Some("@import" | "@embedFile")) {
                        let literal = named.next().and_then(|arguments| {
                            let mut children = arguments.walk();
                            let mut values = arguments.named_children(&mut children);
                            let value = values.next()?;
                            (values.next().is_none() && value.kind() == "string")
                                .then(|| string(value, content))
                                .flatten()
                        });
                        let module = literal
                            .map(|module| {
                                if builtin == Some("@embedFile") && !module.starts_with('.') {
                                    format!("./{module}")
                                } else {
                                    module
                                }
                            })
                            .unwrap_or_else(|| "<computed import>".into());
                        let alias = import_alias(node, content).into_iter().collect();
                        result.imports.push(Import {
                            is_relative: is_file_import(&module),
                            module,
                            items: alias,
                            line: node.start_position().row + 1,
                        });
                    }
                }
                "function_declaration" if include_functions => {
                    if let Some(name) = node.child_by_field_name("name") {
                        let header = &content[node.start_byte()..name.start_byte()];
                        result.exported_functions.push(FunctionDefinition {
                            name: text(name, content).into(),
                            is_exported: header
                                .split_whitespace()
                                .any(|word| matches!(word, "pub" | "export")),
                            line: node.start_position().row + 1,
                        });
                    }
                }
                "call_expression" if include_calls => {
                    if let Some(function) = node.child_by_field_name("function") {
                        let (name, module) = if function.kind() == "field_expression" {
                            (
                                function.child_by_field_name("member"),
                                function
                                    .child_by_field_name("object")
                                    .map(|object| text(object, content).to_owned()),
                            )
                        } else {
                            (Some(function), None)
                        };
                        if let Some(name) = name.filter(|name| name.kind() == "identifier") {
                            result.function_calls.push(FunctionCall {
                                name: text(name, content).into(),
                                module,
                                line: node.start_position().row + 1,
                            });
                        }
                    }
                }
                _ => {}
            }
            if include_types {
                for index in 0..node.child_count() {
                    if node.field_name_for_child(index as u32) == Some("type") {
                        if let Some(value) = node.child(index as u32) {
                            collect_types(value, content, &mut result.type_references);
                        }
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
        let aliases: BTreeMap<_, _> = result
            .imports
            .iter()
            .flat_map(|import| {
                import
                    .items
                    .iter()
                    .map(|alias| (alias.as_str(), import.module.as_str()))
            })
            .collect();
        for call in &mut result.function_calls {
            if let Some(module) = call.module.as_deref().and_then(|alias| aliases.get(alias)) {
                call.module = Some((*module).into());
            }
        }
        let local_definition = if local_types.is_empty() {
            None
        } else {
            validate_import_path(&context.base_dir, path).ok()
        };
        for reference in &mut result.type_references {
            if reference.module.is_none() && local_types.contains(&reference.name) {
                reference.definition_path = local_definition.clone();
                continue;
            }
            let alias = reference.module.as_deref().unwrap_or(&reference.name);
            if let Some(module) = aliases.get(alias) {
                reference.module = Some((*module).into());
                if let Ok(target) =
                    ZigModuleResolver.resolve_import(module, path, &context.base_dir)
                {
                    reference.is_external = target.is_external;
                    if target.is_external {
                        reference.external_package = Some((*module).into());
                    } else {
                        reference.definition_path = Some(target.path);
                    }
                }
            }
        }
        if !include_imports {
            result.imports.clear();
        }
        Ok(result)
    }
    fn can_handle_extension(&self, extension: &str) -> bool {
        extension == "zig"
    }
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["zig"]
    }
}

fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

fn string(node: Node<'_>, source: &str) -> Option<String> {
    // Zig escapes differ from JSON. Keep escaped paths unresolved instead of guessing.
    let value = text(node, source).strip_prefix('"')?.strip_suffix('"')?;
    (!value.contains('\\')).then(|| value.to_owned())
}

fn import_alias(node: Node<'_>, source: &str) -> Option<String> {
    let mut parent = node.parent();
    while let Some(node) = parent {
        if node.kind() == "variable_declaration" {
            let mut children = node.walk();
            return node
                .named_children(&mut children)
                .find(|child| child.kind() == "identifier")
                .map(|name| text(name, source).into());
        }
        if node.kind() == "function_declaration" {
            return None;
        }
        parent = node.parent();
    }
    None
}

fn collect_types(node: Node<'_>, source: &str, references: &mut Vec<TypeReference>) {
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        let (name, module) = match node.kind() {
            "identifier" => (Some(node), None),
            "field_expression" => (
                node.child_by_field_name("member"),
                node.child_by_field_name("object")
                    .map(|value| text(value, source).to_owned()),
            ),
            "builtin_function" => continue,
            _ => {
                let mut children = node.walk();
                pending.extend(node.named_children(&mut children));
                continue;
            }
        };
        if let Some(name) = name {
            let name = text(name, source);
            if matches!(
                name,
                "void"
                    | "bool"
                    | "type"
                    | "anytype"
                    | "anyopaque"
                    | "noreturn"
                    | "usize"
                    | "isize"
                    | "comptime_int"
                    | "comptime_float"
            ) || name
                .strip_prefix(['u', 'i', 'f'])
                .is_some_and(|size| !size.is_empty() && size.chars().all(|ch| ch.is_ascii_digit()))
            {
                continue;
            }
            let reference = TypeReference {
                name: name.into(),
                module,
                line: node.start_position().row + 1,
                definition_path: None,
                is_external: false,
                external_package: None,
            };
            if !references.contains(&reference) {
                references.push(reference);
            }
        }
    }
}

fn is_file_import(module: &str) -> bool {
    module.contains('/') || Path::new(module).extension().is_some()
}

pub struct ZigModuleResolver;

impl ModuleResolver for ZigModuleResolver {
    fn resolve_import(
        &self,
        module: &str,
        from: &Path,
        root: &Path,
    ) -> SemanticResult<ResolvedPath> {
        if module.is_empty()
            || module.len() > 4096
            || module.contains(['\0', '\n', '\r', '\\', ':'])
        {
            return Err(ContextCreatorError::SecurityError(
                "Invalid Zig import path".into(),
            ));
        }
        if !is_file_import(module) {
            validate_module_name(module)?;
            // Named modules are supplied by the compiler/build graph, including root and builtin.
            return Ok(ResolvedPath {
                path: PathBuf::from(module),
                is_external: true,
                confidence: 0.0,
            });
        }
        let path = Path::new(module);
        if path.is_absolute()
            || path
                .components()
                .any(|part| matches!(part, Component::Prefix(_) | Component::RootDir))
        {
            return Err(ContextCreatorError::SecurityError(
                "Zig file imports must be relative".into(),
            ));
        }
        let target = from
            .parent()
            .ok_or_else(|| ContextCreatorError::InvalidPath("Zig source has no parent".into()))?
            .join(path);
        let canonical = validate_import_path(root, &target)?;
        for part in target.ancestors().take_while(|part| *part != root) {
            if std::fs::symlink_metadata(part)
                .map_err(ContextCreatorError::IoError)?
                .file_type()
                .is_symlink()
            {
                return Err(ContextCreatorError::SecurityError(
                    "Zig imports cannot follow symlinks".into(),
                ));
            }
        }
        let target = canonical;
        if !target.is_file() {
            return Err(ContextCreatorError::InvalidPath(
                "Zig import requires a file".into(),
            ));
        }
        Ok(ResolvedPath {
            path: target,
            is_external: false,
            confidence: 1.0,
        })
    }
    fn get_file_extensions(&self) -> Vec<&'static str> {
        vec!["zig", "zon"]
    }
    fn is_external_module(&self, module: &str) -> bool {
        !is_file_import(module)
    }
}
