//! Java source analysis and root-confined package resolution.

use crate::core::semantic::{
    analyzer::{
        AnalysisResult, FunctionCall, FunctionDefinition, Import, LanguageAnalyzer,
        SemanticContext, SemanticResult, TypeReference,
    },
    path_validator::validate_import_path,
    resolver::{ModuleResolver, ResolvedPath},
};
use crate::utils::error::ContextCreatorError;
use std::path::Path;
use tree_sitter::{Node, Parser};

#[derive(Default)]
pub struct JavaAnalyzer;

impl JavaAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageAnalyzer for JavaAnalyzer {
    fn language_name(&self) -> &'static str {
        "Java"
    }

    fn analyze_file(
        &self,
        path: &Path,
        content: &str,
        context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_java::language())
            .map_err(|error| {
                ContextCreatorError::ParseError(format!("Failed to set Java language: {error}"))
            })?;
        parser.set_timeout_micros(5_000_000);
        let tree = parser.parse(content, None).ok_or_else(|| {
            ContextCreatorError::ParseError("Failed to parse Java source".to_string())
        })?;
        let mut result = AnalysisResult::default();
        let mut package_name = String::new();
        if tree.root_node().has_error() {
            result
                .errors
                .push("Java source contains syntax errors".to_string());
        }

        // A cursor avoids recursive traversal on deeply nested, untrusted source.
        let mut cursor = tree.walk();
        'walk: loop {
            let node = cursor.node();
            match node.kind() {
                "package_declaration" => {
                    let mut children = node.walk();
                    let name = node
                        .named_children(&mut children)
                        .find(|child| matches!(child.kind(), "scoped_identifier" | "identifier"));
                    if let Some(name) = name {
                        package_name = node_text(name, content).to_string();
                    }
                }
                "import_declaration" => {
                    let mut children = node.walk();
                    if let Some(imported) = node
                        .named_children(&mut children)
                        .find(|child| matches!(child.kind(), "scoped_identifier" | "identifier"))
                    {
                        let full_name = node_text(imported, content);
                        let is_static = (0..node.child_count()).any(|index| {
                            node.child(index)
                                .is_some_and(|child| child.kind() == "static")
                        });
                        let wildcard = node_text(node, content)
                            .trim_end_matches(';')
                            .trim_end()
                            .ends_with('*');
                        let (module, item) = if is_static && !wildcard {
                            full_name.rsplit_once('.').unwrap_or((full_name, full_name))
                        } else {
                            (
                                full_name,
                                if wildcard {
                                    "*"
                                } else {
                                    full_name.rsplit('.').next().unwrap_or(full_name)
                                },
                            )
                        };
                        result.imports.push(Import {
                            module: module.to_string(),
                            items: vec![item.to_string()],
                            is_relative: false,
                            line: node.start_position().row,
                        });
                    };
                }
                "method_invocation" | "object_creation_expression" => {
                    let field = if node.kind() == "method_invocation" {
                        "name"
                    } else {
                        "type"
                    };
                    if let Some(name) = node.child_by_field_name(field) {
                        result.function_calls.push(FunctionCall {
                            name: node_text(name, content).to_string(),
                            module: node
                                .child_by_field_name("object")
                                .map(|object| node_text(object, content).to_string()),
                            line: node.start_position().row,
                        });
                    }
                }
                "method_declaration" | "constructor_declaration" => {
                    if let Some(name) = node.child_by_field_name("name") {
                        let mut children = node.walk();
                        let modifiers = node
                            .named_children(&mut children)
                            .find(|child| child.kind() == "modifiers")
                            .map(|child| node_text(child, content))
                            .unwrap_or_default();
                        result.exported_functions.push(FunctionDefinition {
                            name: node_text(name, content).to_string(),
                            is_exported: !modifiers
                                .split_whitespace()
                                .any(|word| word == "private"),
                            line: node.start_position().row,
                        });
                    }
                }
                "type_identifier" => result.type_references.push(TypeReference {
                    name: node_text(node, content).to_string(),
                    module: None,
                    line: node.start_position().row,
                    definition_path: None,
                    is_external: false,
                    external_package: None,
                }),
                _ => {}
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

        let resolver = JavaModuleResolver;
        for reference in &mut result.type_references {
            for import in &result.imports {
                let wildcard = import.items.iter().any(|item| item == "*");
                let candidate = if wildcard {
                    format!("{}.{}", import.module, reference.name)
                } else if import.module.rsplit('.').next() == Some(reference.name.as_str()) {
                    import.module.clone()
                } else {
                    continue;
                };
                let resolved = resolver.resolve_import(&candidate, path, &context.base_dir)?;
                if wildcard && resolved.is_external {
                    continue;
                }
                reference.module = Some(candidate);
                reference.is_external = resolved.is_external;
                if !resolved.is_external {
                    reference.definition_path = Some(resolved.path);
                }
                break;
            }
            if reference.definition_path.is_none() && reference.module.is_none() {
                if let Some(parent) = path.parent() {
                    let sibling = parent.join(format!("{}.java", reference.name));
                    if sibling.is_file() {
                        reference.definition_path =
                            Some(validate_import_path(&context.base_dir, &sibling)?);
                        reference.module = Some(if package_name.is_empty() {
                            reference.name.clone()
                        } else {
                            format!("{package_name}.{}", reference.name)
                        });
                    }
                }
            }
        }
        for reference in &result.type_references {
            if reference
                .definition_path
                .as_deref()
                .is_some_and(|definition| {
                    definition != path && path.canonicalize().ok().as_deref() != Some(definition)
                })
            {
                if let Some(module) = &reference.module {
                    if !result.imports.iter().any(|import| &import.module == module) {
                        result.imports.push(Import {
                            module: module.clone(),
                            items: vec![reference.name.clone()],
                            is_relative: false,
                            line: reference.line,
                        });
                    }
                }
            }
        }
        for call in &mut result.function_calls {
            if let Some(module) = &call.module {
                if let Some(import) = result
                    .imports
                    .iter()
                    .find(|import| import.module.rsplit('.').next() == Some(module.as_str()))
                {
                    call.module = Some(import.module.clone());
                }
            }
        }
        Ok(result)
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        extension == "java"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["java"]
    }
}

fn node_text<'a>(node: Node<'_>, content: &'a str) -> &'a str {
    content.get(node.byte_range()).unwrap_or_default()
}

pub struct JavaModuleResolver;

impl ModuleResolver for JavaModuleResolver {
    fn resolve_import(
        &self,
        module_path: &str,
        from_file: &Path,
        base_dir: &Path,
    ) -> Result<ResolvedPath, ContextCreatorError> {
        let components: Vec<_> = module_path.split('.').collect();
        if components.iter().any(|part| {
            part.is_empty()
                || !part
                    .chars()
                    .all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '$')
        }) {
            return Err(ContextCreatorError::SecurityError(
                "Invalid Java package name".to_string(),
            ));
        }
        let base = base_dir.canonicalize().map_err(|error| {
            ContextCreatorError::InvalidPath(format!("Invalid Java project root: {error}"))
        })?;
        let from = validate_import_path(&base, from_file)?;
        let mut roots = vec![
            base.join("src/main/java"),
            base.join("src/test/java"),
            base.join("src"),
            base.clone(),
        ];
        // Supports module-specific source roots without an additional tree walk.
        roots.extend(
            from.parent()
                .into_iter()
                .flat_map(Path::ancestors)
                .take_while(|parent| parent.starts_with(&base))
                .map(Path::to_path_buf),
        );
        for root in roots {
            // Nested classes resolve to their enclosing source file.
            for end in (1..=components.len()).rev() {
                let candidate = root
                    .join(components[..end].join("/"))
                    .with_extension("java");
                if candidate.is_file() {
                    return Ok(ResolvedPath {
                        path: validate_import_path(&base, &candidate)?,
                        is_external: false,
                        confidence: 1.0,
                    });
                }
            }
        }
        Ok(ResolvedPath {
            path: module_path.into(),
            is_external: true,
            confidence: 0.5,
        })
    }

    fn get_file_extensions(&self) -> Vec<&'static str> {
        vec!["java"]
    }

    fn is_external_module(&self, module_path: &str) -> bool {
        module_path.starts_with("java.")
            || module_path.starts_with("javax.")
            || module_path.starts_with("jdk.")
    }
}
