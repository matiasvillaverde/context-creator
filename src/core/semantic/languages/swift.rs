//! Semantic analyzer for Swift and Swift Package Manager projects.

use crate::core::semantic::{
    analyzer::{
        AnalysisResult, FunctionCall, Import, LanguageAnalyzer, SemanticContext, SemanticResult,
        TypeReference,
    },
    path_validator::{validate_import_path, validate_module_name},
    query_engine::QueryEngine,
    resolver::{ModuleResolver, ResolvedPath},
};
use crate::utils::error::ContextCreatorError;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tree_sitter::{Node, Parser, Query, QueryCursor};

#[allow(clippy::new_without_default)]
pub struct SwiftAnalyzer;

impl SwiftAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

fn swift_query_engine() -> &'static QueryEngine {
    static ENGINE: OnceLock<QueryEngine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        QueryEngine::new(tree_sitter_swift::language(), "swift")
            .expect("Failed to create Swift query engine")
    })
}

fn swift_module_type_cache() -> &'static Mutex<HashMap<PathBuf, HashMap<String, PathBuf>>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, HashMap<String, PathBuf>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn swift_manifest_target_path_cache() -> &'static Mutex<HashMap<PathBuf, HashMap<String, PathBuf>>>
{
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, HashMap<String, PathBuf>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

impl LanguageAnalyzer for SwiftAnalyzer {
    fn language_name(&self) -> &'static str {
        "Swift"
    }

    fn analyze_file(
        &self,
        path: &Path,
        content: &str,
        context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        self.analyze_requested(path, content, context, true, true, true, true)
    }

    #[allow(clippy::too_many_arguments)]
    fn analyze_requested(
        &self,
        path: &Path,
        content: &str,
        context: &SemanticContext,
        include_imports: bool,
        include_function_calls: bool,
        include_type_references: bool,
        include_function_definitions: bool,
    ) -> SemanticResult<AnalysisResult> {
        if include_imports
            && !include_function_calls
            && !include_type_references
            && !include_function_definitions
        {
            let mut result = AnalysisResult {
                imports: extract_swift_imports_fast(content),
                ..AnalysisResult::default()
            };
            self.normalize_imports(&mut result);
            return Ok(result);
        }

        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_swift::language())
            .map_err(|e| {
                ContextCreatorError::ParseError(format!("Failed to set Swift language: {e}"))
            })?;

        let mut result = swift_query_engine().analyze_with_parser(&mut parser, content)?;
        result.imports = extract_swift_imports_fast(content);
        self.normalize_imports(&mut result);

        if include_function_calls {
            result.function_calls = extract_swift_function_calls(content)?;
        } else {
            result.function_calls.clear();
        }

        if include_type_references {
            self.merge_swift_type_references(&mut result, content)?;
            self.correlate_qualified_symbols_with_imports(&mut result);
            self.dedupe_type_references(&mut result);
            self.resolve_swift_type_definitions(
                &mut result.type_references,
                &result.imports,
                path,
                &context.base_dir,
            )?;
        } else {
            result.type_references.clear();
        }

        if !include_imports {
            result.imports.clear();
        }
        if !include_function_definitions {
            result.exported_functions.clear();
        }

        Ok(result)
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        extension == "swift"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["swift"]
    }
}

impl SwiftAnalyzer {
    fn normalize_imports(&self, result: &mut AnalysisResult) {
        let mut merged: Vec<Import> = Vec::new();
        let mut index_by_module: HashMap<String, usize> = HashMap::new();

        for mut import in result.imports.drain(..) {
            let clean_module = clean_swift_import_path(&import.module);
            let (module, imported_item) = split_swift_import_module_and_item(&clean_module);
            import.module = module;
            if let Some(item) = imported_item {
                if !import.items.contains(&item) {
                    import.items.push(item);
                }
            }

            if let Some(index) = index_by_module.get(&import.module).copied() {
                let existing = &mut merged[index];
                for item in import.items {
                    if !existing.items.contains(&item) {
                        existing.items.push(item);
                    }
                }
            } else {
                index_by_module.insert(import.module.clone(), merged.len());
                merged.push(import);
            }
        }

        result.imports = merged;
    }

    fn merge_swift_type_references(
        &self,
        result: &mut AnalysisResult,
        content: &str,
    ) -> Result<(), ContextCreatorError> {
        let mut type_refs = extract_swift_user_type_references(content)?;

        for call in &result.function_calls {
            if is_swift_nominal_type_name(&call.name) && !is_swift_builtin_type(&call.name) {
                type_refs.push(TypeReference {
                    name: call.name.clone(),
                    module: call.module.clone(),
                    line: call.line,
                    definition_path: None,
                    is_external: false,
                    external_package: None,
                });
            }
        }

        result.type_references.extend(type_refs);
        Ok(())
    }

    fn correlate_qualified_symbols_with_imports(&self, result: &mut AnalysisResult) {
        let imported_modules = swift_import_modules(&result.imports);

        for type_ref in &mut result.type_references {
            if let Some(module) = type_ref.module.as_mut() {
                let primary = swift_primary_module(module);
                if imported_modules.contains(&primary) {
                    *module = primary;
                }
            }
        }

        for call in &mut result.function_calls {
            if let Some(module) = call.module.as_mut() {
                let primary = swift_primary_module(module);
                if imported_modules.contains(&primary) {
                    *module = primary;
                }
            }
        }
    }

    fn dedupe_type_references(&self, result: &mut AnalysisResult) {
        let mut deduped = Vec::new();
        let mut seen = HashSet::new();

        for type_ref in result.type_references.drain(..) {
            if is_swift_builtin_type(&type_ref.name) {
                continue;
            }

            let key = (
                type_ref.name.clone(),
                type_ref.module.clone(),
                type_ref.line,
            );
            if seen.insert(key) {
                deduped.push(type_ref);
            }
        }

        result.type_references = deduped;
    }

    fn resolve_swift_type_definitions(
        &self,
        type_refs: &mut [TypeReference],
        imports: &[Import],
        current_file: &Path,
        project_root: &Path,
    ) -> Result<(), ContextCreatorError> {
        let resolver = SwiftModuleResolver;
        let search_context = SwiftTypeSearchContext {
            resolver: &resolver,
            current_file,
            project_root,
        };
        let imported_modules = swift_import_modules(imports);
        let mut module_type_cache = HashMap::new();

        for type_ref in type_refs.iter_mut() {
            if type_ref.definition_path.is_some()
                || type_ref.is_external
                || is_swift_builtin_type(&type_ref.name)
            {
                continue;
            }

            let definition = if let Some(module) = type_ref.module.as_deref() {
                let module = swift_primary_module(module);
                self.find_type_in_swift_module(
                    &search_context,
                    &module,
                    &type_ref.name,
                    &mut module_type_cache,
                )?
            } else {
                let same_target = find_swift_type_definition_in_current_target(
                    current_file,
                    &type_ref.name,
                    project_root,
                    &mut module_type_cache,
                )?;
                if same_target.is_some() {
                    same_target
                } else {
                    self.find_type_in_imported_swift_modules(
                        &search_context,
                        &imported_modules,
                        &type_ref.name,
                        &mut module_type_cache,
                    )?
                }
            };

            if let Some(path) = definition {
                match validate_import_path(project_root, &path) {
                    Ok(validated_path) => type_ref.definition_path = Some(validated_path),
                    Err(_) => type_ref.is_external = true,
                }
            }
        }

        Ok(())
    }

    fn find_type_in_imported_swift_modules(
        &self,
        context: &SwiftTypeSearchContext<'_>,
        imported_modules: &[String],
        type_name: &str,
        module_type_cache: &mut HashMap<PathBuf, HashMap<String, PathBuf>>,
    ) -> Result<Option<PathBuf>, ContextCreatorError> {
        for module in imported_modules {
            if context.resolver.is_external_module(module) {
                continue;
            }

            if let Some(path) =
                self.find_type_in_swift_module(context, module, type_name, module_type_cache)?
            {
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    fn find_type_in_swift_module(
        &self,
        context: &SwiftTypeSearchContext<'_>,
        module: &str,
        type_name: &str,
        module_type_cache: &mut HashMap<PathBuf, HashMap<String, PathBuf>>,
    ) -> Result<Option<PathBuf>, ContextCreatorError> {
        match context
            .resolver
            .resolve_import(module, context.current_file, context.project_root)
        {
            Ok(resolved) if resolved.is_external => Ok(None),
            Ok(resolved) => {
                let search_root = swift_target_dir_for_file(&resolved.path, context.project_root)
                    .unwrap_or_else(|| resolved.path.clone());
                find_swift_type_definition_in_module(&search_root, type_name, module_type_cache)
            }
            Err(_) => Ok(None),
        }
    }
}

struct SwiftTypeSearchContext<'a> {
    resolver: &'a SwiftModuleResolver,
    current_file: &'a Path,
    project_root: &'a Path,
}

pub struct SwiftModuleResolver;

impl ModuleResolver for SwiftModuleResolver {
    fn resolve_import(
        &self,
        module_path: &str,
        from_file: &Path,
        base_dir: &Path,
    ) -> Result<ResolvedPath, ContextCreatorError> {
        let clean_module = clean_swift_import_path(module_path);
        let module = swift_primary_module(&clean_module);
        validate_module_name(&module)?;

        let package_root = find_swift_package_root(from_file, base_dir);
        for candidate in swift_module_candidate_dirs(&module, from_file, &package_root, base_dir) {
            if let Some(path) = find_swift_module_file(&candidate, &module) {
                return resolved_swift_path(path, base_dir, false, 0.95);
            }
        }

        Ok(ResolvedPath {
            path: package_root.join("Package.swift"),
            is_external: true,
            confidence: if self.is_external_module(&module) {
                0.95
            } else {
                0.5
            },
        })
    }

    fn get_file_extensions(&self) -> Vec<&'static str> {
        vec!["swift"]
    }

    fn is_external_module(&self, module_path: &str) -> bool {
        let module = swift_primary_module(&clean_swift_import_path(module_path));
        matches!(
            module.as_str(),
            "Swift"
                | "Foundation"
                | "FoundationNetworking"
                | "PackageDescription"
                | "Darwin"
                | "Glibc"
                | "Dispatch"
                | "CoreFoundation"
                | "CoreGraphics"
                | "UIKit"
                | "AppKit"
                | "SwiftUI"
                | "Combine"
                | "XCTest"
                | "Observation"
                | "CoreData"
                | "MapKit"
                | "WebKit"
                | "AVFoundation"
                | "Security"
                | "OSLog"
                | "CloudKit"
                | "Network"
                | "CryptoKit"
                | "Metal"
                | "SpriteKit"
                | "SceneKit"
                | "WidgetKit"
        )
    }
}

fn extract_swift_imports_fast(content: &str) -> Vec<Import> {
    const IMPORT_KINDS: &[&str] = &[
        "class",
        "struct",
        "enum",
        "protocol",
        "typealias",
        "func",
        "var",
        "let",
    ];
    const IMPORT_MODIFIERS: &[&str] = &["public", "package", "internal", "private", "fileprivate"];

    let mut imports = Vec::new();

    for (line_index, raw_line) in content.lines().enumerate() {
        let Some(line) = raw_line.split("//").next() else {
            continue;
        };

        for raw_statement in line.split(';') {
            let mut statement = raw_statement.trim();
            if statement.is_empty() {
                continue;
            }

            loop {
                if statement.starts_with('@') {
                    let Some((_, rest)) = statement.split_once(char::is_whitespace) else {
                        statement = "";
                        break;
                    };
                    statement = rest.trim_start();
                    continue;
                }

                if let Some(modifier) = IMPORT_MODIFIERS.iter().find(|modifier| {
                    statement
                        .strip_prefix(**modifier)
                        .is_some_and(|rest| rest.chars().next().is_some_and(char::is_whitespace))
                }) {
                    statement = statement
                        .strip_prefix(modifier)
                        .unwrap_or(statement)
                        .trim_start();
                    continue;
                }

                break;
            }

            let Some(rest) = statement.strip_prefix("import") else {
                continue;
            };
            if !rest
                .chars()
                .next()
                .is_some_and(|ch| ch.is_whitespace() || ch == ';')
            {
                continue;
            }

            let rest = rest.trim_start_matches(|ch: char| ch.is_whitespace() || ch == ';');
            let mut parts = rest.split_whitespace();
            let Some(first) = parts.next() else {
                continue;
            };

            let module_path = if IMPORT_KINDS.contains(&first) {
                parts.next()
            } else {
                Some(first)
            };

            let Some(module_path) = module_path else {
                continue;
            };

            let module = module_path
                .trim_end_matches(';')
                .trim_matches('"')
                .trim_matches('\'')
                .trim_matches('`')
                .to_string();

            if module.is_empty() {
                continue;
            }

            imports.push(Import {
                module,
                items: Vec::new(),
                is_relative: false,
                line: line_index + 1,
            });
        }
    }

    imports
}

fn extract_swift_user_type_references(
    content: &str,
) -> Result<Vec<TypeReference>, ContextCreatorError> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_swift::language())
        .map_err(|e| {
            ContextCreatorError::ParseError(format!("Failed to set Swift language: {e}"))
        })?;

    let Some(tree) = parser.parse(content, None) else {
        return Ok(Vec::new());
    };

    let query = Query::new(tree_sitter_swift::language(), "(user_type) @type").map_err(|e| {
        ContextCreatorError::ParseError(format!("Failed to create Swift type query: {e}"))
    })?;
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

    let mut refs = Vec::new();
    let mut seen = HashSet::new();
    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let type_identifiers = direct_child_type_identifiers(node, content);
            if type_identifiers.is_empty() {
                continue;
            }

            let name = type_identifiers.last().cloned().unwrap_or_default();
            if name.is_empty() || is_swift_builtin_type(&name) {
                continue;
            }

            let module = if type_identifiers.len() > 1 {
                type_identifiers.first().cloned()
            } else {
                None
            };
            let key = (name.clone(), module.clone(), node.start_position().row + 1);
            if seen.insert(key) {
                refs.push(TypeReference {
                    name,
                    module,
                    line: node.start_position().row + 1,
                    definition_path: None,
                    is_external: false,
                    external_package: None,
                });
            }
        }
    }

    Ok(refs)
}

fn extract_swift_function_calls(content: &str) -> Result<Vec<FunctionCall>, ContextCreatorError> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_swift::language())
        .map_err(|e| {
            ContextCreatorError::ParseError(format!("Failed to set Swift language: {e}"))
        })?;

    let Some(tree) = parser.parse(content, None) else {
        return Ok(Vec::new());
    };

    let query =
        Query::new(tree_sitter_swift::language(), "(call_expression) @call").map_err(|e| {
            ContextCreatorError::ParseError(format!("Failed to create Swift call query: {e}"))
        })?;
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

    let mut calls = Vec::new();
    let mut seen = HashSet::new();
    for match_ in matches {
        for capture in match_.captures {
            if let Some(call) = swift_call_from_node(capture.node, content) {
                let key = (call.name.clone(), call.module.clone(), call.line);
                if seen.insert(key) {
                    calls.push(call);
                }
            }
        }
    }

    Ok(calls)
}

fn swift_call_from_node(node: Node<'_>, content: &str) -> Option<FunctionCall> {
    let target = first_call_target(node)?;
    let line = node.start_position().row + 1;

    match target.kind() {
        "simple_identifier" => Some(FunctionCall {
            name: node_text(target, content)?,
            module: None,
            line,
        }),
        "user_type" => Some(FunctionCall {
            name: direct_child_type_identifiers(target, content).pop()?,
            module: None,
            line,
        }),
        "navigation_expression" => {
            let suffix = target.child_by_field_name("suffix")?;
            let name = navigation_suffix_name(suffix, content)?;
            let module = target
                .child_by_field_name("target")
                .and_then(|module_node| swift_call_module_from_target(module_node, content));

            Some(FunctionCall { name, module, line })
        }
        _ => None,
    }
}

fn first_call_target<'a>(node: Node<'a>) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let target = node
        .named_children(&mut cursor)
        .find(|child| child.kind() != "call_suffix");
    target
}

fn navigation_suffix_name(node: Node<'_>, content: &str) -> Option<String> {
    if let Some(suffix) = node.child_by_field_name("suffix") {
        return node_text(suffix, content);
    }

    let mut cursor = node.walk();
    let name = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == "simple_identifier")
        .and_then(|child| node_text(child, content));
    name
}

fn swift_call_module_from_target(node: Node<'_>, content: &str) -> Option<String> {
    match node.kind() {
        "simple_identifier" => node_text(node, content),
        "user_type" => direct_child_type_identifiers(node, content)
            .first()
            .cloned(),
        "navigation_expression" => node
            .child_by_field_name("suffix")
            .and_then(|suffix| navigation_suffix_name(suffix, content)),
        _ => None,
    }
}

fn direct_child_type_identifiers(node: Node<'_>, content: &str) -> Vec<String> {
    let mut identifiers = Vec::new();
    let mut cursor = node.walk();

    for child in node.named_children(&mut cursor) {
        if child.kind() == "type_identifier" {
            if let Ok(text) = child.utf8_text(content.as_bytes()) {
                identifiers.push(text.to_string());
            }
        }
    }

    identifiers
}

fn node_text(node: Node<'_>, content: &str) -> Option<String> {
    node.utf8_text(content.as_bytes())
        .ok()
        .map(ToOwned::to_owned)
}

fn find_swift_type_definition_in_current_target(
    current_file: &Path,
    type_name: &str,
    project_root: &Path,
    module_type_cache: &mut HashMap<PathBuf, HashMap<String, PathBuf>>,
) -> Result<Option<PathBuf>, ContextCreatorError> {
    if swift_file_contains_type_definition(current_file, type_name)? {
        return Ok(Some(current_file.to_path_buf()));
    }

    let Some(target_dir) = swift_target_dir_for_file(current_file, project_root) else {
        return Ok(None);
    };

    find_swift_type_definition_in_module(&target_dir, type_name, module_type_cache)
}

fn find_swift_type_definition_in_module(
    module_path: &Path,
    type_name: &str,
    module_type_cache: &mut HashMap<PathBuf, HashMap<String, PathBuf>>,
) -> Result<Option<PathBuf>, ContextCreatorError> {
    let search_root = if module_path.is_file() {
        module_path.parent().unwrap_or(module_path)
    } else {
        module_path
    };
    let cache_key = search_root
        .canonicalize()
        .unwrap_or_else(|_| search_root.to_path_buf());

    if !module_type_cache.contains_key(&cache_key) {
        if let Some(definitions) = swift_module_type_cache()
            .lock()
            .ok()
            .and_then(|cache| cache.get(&cache_key).cloned())
        {
            module_type_cache.insert(cache_key.clone(), definitions);
        } else {
            let definitions = collect_swift_type_definitions(module_path)?;
            if let Ok(mut cache) = swift_module_type_cache().lock() {
                cache.insert(cache_key.clone(), definitions.clone());
            }
            module_type_cache.insert(cache_key.clone(), definitions);
        }
    }

    Ok(module_type_cache
        .get(&cache_key)
        .and_then(|definitions| definitions.get(type_name).cloned()))
}

fn collect_swift_type_definitions(
    module_path: &Path,
) -> Result<HashMap<String, PathBuf>, ContextCreatorError> {
    let module_dir = if module_path.is_file() {
        module_path.parent().unwrap_or(module_path)
    } else {
        module_path
    };

    let mut candidates = Vec::new();
    if module_path.is_file() {
        candidates.push(module_path.to_path_buf());
    }

    candidates.extend(swift_source_files(module_dir));

    let mut definitions = HashMap::new();
    let mut seen = HashSet::new();
    for candidate in candidates {
        if !seen.insert(candidate.clone()) {
            continue;
        }

        if candidate.extension().and_then(|ext| ext.to_str()) != Some("swift") {
            continue;
        }

        let content = fs::read_to_string(&candidate).map_err(|e| {
            ContextCreatorError::ReadError(format!(
                "Failed to read Swift file {}: {e}",
                candidate.display()
            ))
        })?;

        for type_name in swift_type_definitions_in_content(&content)? {
            definitions
                .entry(type_name)
                .or_insert_with(|| candidate.clone());
        }
    }

    Ok(definitions)
}

fn swift_file_contains_type_definition(
    path: &Path,
    type_name: &str,
) -> Result<bool, ContextCreatorError> {
    if path.extension().and_then(|ext| ext.to_str()) != Some("swift") {
        return Ok(false);
    }

    let content = fs::read_to_string(path).map_err(|e| {
        ContextCreatorError::ReadError(format!("Failed to read Swift file {}: {e}", path.display()))
    })?;

    Ok(swift_type_definitions_in_content(&content)?
        .iter()
        .any(|definition| definition == type_name))
}

fn swift_type_definitions_in_content(content: &str) -> Result<Vec<String>, ContextCreatorError> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_swift::language())
        .map_err(|e| {
            ContextCreatorError::ParseError(format!("Failed to set Swift language: {e}"))
        })?;

    let Some(tree) = parser.parse(content, None) else {
        return Ok(Vec::new());
    };

    let query_text = r#"
        [
          (class_declaration name: (type_identifier) @name)
          (protocol_declaration name: (type_identifier) @name)
          (typealias_declaration name: (type_identifier) @name)
        ]
    "#;
    let query = Query::new(tree_sitter_swift::language(), query_text).map_err(|e| {
        ContextCreatorError::ParseError(format!("Failed to create Swift definition query: {e}"))
    })?;
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

    let mut definitions = Vec::new();
    let mut seen = HashSet::new();
    for match_ in matches {
        for capture in match_.captures {
            if let Ok(captured_text) = capture.node.utf8_text(content.as_bytes()) {
                let name = captured_text.to_string();
                if seen.insert(name.clone()) {
                    definitions.push(name);
                }
            }
        }
    }

    Ok(definitions)
}

fn swift_module_candidate_dirs(
    module: &str,
    from_file: &Path,
    package_root: &Path,
    base_dir: &Path,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(manifest_path) = swift_manifest_target_paths(package_root).get(module) {
        candidates.push(manifest_path.clone());
    }
    candidates.push(package_root.join("Sources").join(module));
    candidates.push(package_root.join("Tests").join(module));
    candidates.push(package_root.join("Tests").join(format!("{module}Tests")));
    candidates.push(package_root.join(module));
    candidates.push(package_root.join(format!("{module}.swift")));

    if package_root != base_dir {
        candidates.push(base_dir.join("Sources").join(module));
        candidates.push(base_dir.join(module));
        candidates.push(base_dir.join(format!("{module}.swift")));
    }

    if let Some(parent) = from_file.parent() {
        candidates.push(parent.join(module));
        candidates.push(parent.join(format!("{module}.swift")));

        if let Some(sources_dir) = swift_sources_dir_for_file(from_file) {
            candidates.push(sources_dir.join(module));
            candidates.push(sources_dir.join(format!("{module}.swift")));
        }
    }

    dedupe_paths(candidates)
}

fn find_swift_module_file(candidate: &Path, module: &str) -> Option<PathBuf> {
    if candidate.is_file()
        && candidate.extension().and_then(|ext| ext.to_str()) == Some("swift")
        && !is_swift_test_file(candidate)
    {
        return Some(candidate.to_path_buf());
    }

    let direct_file = candidate.with_extension("swift");
    if direct_file.is_file() && !is_swift_test_file(&direct_file) {
        return Some(direct_file);
    }

    if !candidate.is_dir() {
        return None;
    }

    for preferred in [format!("{module}.swift"), "main.swift".to_string()] {
        let path = candidate.join(preferred);
        if path.is_file() && !is_swift_test_file(&path) {
            return Some(path);
        }
    }

    swift_source_files(candidate).into_iter().next()
}

fn swift_source_files(root: &Path) -> Vec<PathBuf> {
    fn visit(dir: &Path, files: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with('.') || name == ".build")
                || path.extension().and_then(|ext| ext.to_str()) == Some("docc")
            {
                continue;
            }

            if path.is_dir() {
                visit(&path, files);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("swift")
                && !is_swift_test_file(&path)
            {
                files.push(path);
            }
        }
    }

    let mut files = Vec::new();
    if root.is_dir() {
        visit(root, &mut files);
    }
    files.sort();
    files
}

pub(crate) fn swift_target_dir_for_file(file: &Path, project_root: &Path) -> Option<PathBuf> {
    let package_root = find_swift_package_root(file, project_root);
    if package_root.join("Package.swift").exists() {
        let mut matching_targets = swift_manifest_target_paths(&package_root)
            .into_values()
            .filter(|target_dir| file.starts_with(target_dir))
            .collect::<Vec<_>>();
        matching_targets.sort_by_key(|path| path.components().count());
        if let Some(target_dir) = matching_targets.pop() {
            return Some(target_dir);
        }
    }

    let relative = file.strip_prefix(project_root).ok()?;
    let components: Vec<_> = relative
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect();

    if components.len() >= 3 && (components[0] == "Sources" || components[0] == "Tests") {
        return Some(project_root.join(&components[0]).join(&components[1]));
    }

    if !package_root.join("Package.swift").exists() {
        return Some(project_root.to_path_buf());
    }

    file.parent().map(Path::to_path_buf)
}

fn swift_manifest_target_paths(package_root: &Path) -> HashMap<String, PathBuf> {
    let cache_key = package_root
        .canonicalize()
        .unwrap_or_else(|_| package_root.to_path_buf());

    if let Some(paths) = swift_manifest_target_path_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get(&cache_key).cloned())
    {
        return paths;
    }

    let paths = parse_swift_manifest_target_paths(package_root);
    if let Ok(mut cache) = swift_manifest_target_path_cache().lock() {
        cache.insert(cache_key, paths.clone());
    }
    paths
}

fn parse_swift_manifest_target_paths(package_root: &Path) -> HashMap<String, PathBuf> {
    let package_file = package_root.join("Package.swift");
    let Ok(content) = fs::read_to_string(package_file) else {
        return HashMap::new();
    };

    let mut target_paths = HashMap::new();
    for call in swift_manifest_target_calls(&content) {
        let Some(name) = swift_call_string_argument(&call, "name") else {
            continue;
        };
        let Some(path) = swift_call_string_argument(&call, "path") else {
            continue;
        };

        target_paths.insert(name, package_root.join(path));
    }

    target_paths
}

fn swift_manifest_target_calls(content: &str) -> Vec<String> {
    const TARGET_MARKERS: &[&str] = &[
        ".target(",
        ".executableTarget(",
        ".testTarget(",
        ".macro(",
        ".plugin(",
    ];

    let mut calls = Vec::new();
    let mut offset = 0;
    while offset < content.len() {
        let next = TARGET_MARKERS
            .iter()
            .filter_map(|marker| {
                content[offset..]
                    .find(marker)
                    .map(|index| (offset + index, marker.len()))
            })
            .min_by_key(|(index, _)| *index);

        let Some((marker_index, marker_len)) = next else {
            break;
        };

        let open_paren = marker_index + marker_len - 1;
        if let Some((call, end_index)) = swift_balanced_parenthesized_content(content, open_paren) {
            calls.push(call);
            offset = end_index;
        } else {
            offset = open_paren + 1;
        }
    }

    calls
}

fn swift_balanced_parenthesized_content(
    content: &str,
    open_paren: usize,
) -> Option<(String, usize)> {
    if content.as_bytes().get(open_paren) != Some(&b'(') {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaping = false;
    let mut start = None;

    for (index, ch) in content[open_paren..].char_indices() {
        let absolute_index = open_paren + index;

        if in_string {
            if escaping {
                escaping = false;
            } else if ch == '\\' {
                escaping = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' => {
                depth += 1;
                if depth == 1 {
                    start = Some(absolute_index + ch.len_utf8());
                }
            }
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    let start = start?;
                    return Some((
                        content[start..absolute_index].to_string(),
                        absolute_index + 1,
                    ));
                }
            }
            _ => {}
        }
    }

    None
}

fn swift_call_string_argument(call: &str, label: &str) -> Option<String> {
    let label = format!("{label}:");
    let label_index = call.find(&label)?;
    let after_label = &call[label_index + label.len()..];
    let quote_start = after_label.find('"')? + label_index + label.len();
    let value_start = quote_start + 1;

    let mut escaping = false;
    for (index, ch) in call[value_start..].char_indices() {
        if escaping {
            escaping = false;
        } else if ch == '\\' {
            escaping = true;
        } else if ch == '"' {
            return Some(call[value_start..value_start + index].to_string());
        }
    }

    None
}

fn swift_sources_dir_for_file(file: &Path) -> Option<PathBuf> {
    let mut current = file.parent()?;
    loop {
        if current.file_name().and_then(|name| name.to_str()) == Some("Sources") {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

fn find_swift_package_root(from_file: &Path, base_dir: &Path) -> PathBuf {
    let mut current = if from_file.is_file() {
        from_file.parent().unwrap_or(from_file)
    } else {
        from_file
    };

    loop {
        if current.join("Package.swift").exists() {
            return current.to_path_buf();
        }
        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            break;
        }
    }

    base_dir.to_path_buf()
}

fn resolved_swift_path(
    path: PathBuf,
    base_dir: &Path,
    is_external: bool,
    confidence: f32,
) -> Result<ResolvedPath, ContextCreatorError> {
    let path = if is_external {
        path
    } else {
        validate_import_path(base_dir, &path)?
    };

    Ok(ResolvedPath {
        path,
        is_external,
        confidence,
    })
}

fn clean_swift_import_path(module_path: &str) -> String {
    module_path
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('`')
        .to_string()
}

fn split_swift_import_module_and_item(module_path: &str) -> (String, Option<String>) {
    let mut parts = module_path.split('.').filter(|part| !part.is_empty());
    let Some(module) = parts.next() else {
        return (module_path.to_string(), None);
    };

    let item = parts.collect::<Vec<_>>().join(".");
    (
        module.to_string(),
        if item.is_empty() { None } else { Some(item) },
    )
}

fn swift_primary_module(module_path: &str) -> String {
    module_path
        .split('.')
        .find(|part| !part.is_empty())
        .unwrap_or(module_path)
        .to_string()
}

fn swift_import_modules(imports: &[Import]) -> Vec<String> {
    let mut modules = imports
        .iter()
        .map(|import| swift_primary_module(&import.module))
        .collect::<Vec<_>>();
    modules.sort();
    modules.dedup();
    modules
}

fn is_swift_nominal_type_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
}

fn is_swift_builtin_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "String"
            | "Substring"
            | "Int"
            | "Int8"
            | "Int16"
            | "Int32"
            | "Int64"
            | "UInt"
            | "UInt8"
            | "UInt16"
            | "UInt32"
            | "UInt64"
            | "Float"
            | "Double"
            | "CGFloat"
            | "Bool"
            | "Character"
            | "Void"
            | "Any"
            | "AnyObject"
            | "Never"
            | "Array"
            | "Dictionary"
            | "Set"
            | "Optional"
            | "Result"
            | "Data"
            | "Date"
            | "URL"
            | "UUID"
            | "Error"
            | "Task"
    )
}

fn is_swift_test_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with("Tests.swift") || name.ends_with("Test.swift"))
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for path in paths {
        if seen.insert(path.clone()) {
            deduped.push(path);
        }
    }

    deduped
}
