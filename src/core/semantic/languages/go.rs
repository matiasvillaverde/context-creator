//! Semantic analyzer for Go

use crate::core::semantic::{
    analyzer::{
        AnalysisResult, Import, LanguageAnalyzer, SemanticContext, SemanticResult, TypeReference,
    },
    path_validator::{validate_import_path, validate_module_name},
    query_engine::QueryEngine,
    resolver::{ModuleResolver, ResolvedPath},
};
use crate::utils::error::ContextCreatorError;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Parser, Query, QueryCursor};

#[allow(clippy::new_without_default)]
pub struct GoAnalyzer {
    query_engine: QueryEngine,
}

impl GoAnalyzer {
    pub fn new() -> Self {
        let language = tree_sitter_go::language();
        let query_engine =
            QueryEngine::new(language, "go").expect("Failed to create Go query engine");
        Self { query_engine }
    }
}

impl LanguageAnalyzer for GoAnalyzer {
    fn language_name(&self) -> &'static str {
        "Go"
    }

    fn analyze_file(
        &self,
        path: &Path,
        content: &str,
        context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_go::language())
            .map_err(|e| ContextCreatorError::ParseError(format!("Failed to set language: {e}")))?;

        let mut result = self
            .query_engine
            .analyze_with_parser(&mut parser, content)?;

        self.merge_duplicate_imports(&mut result);
        self.correlate_qualified_symbols_with_imports(&mut result);
        self.dedupe_type_references(&mut result);

        self.query_engine.resolve_type_definitions(
            &mut result.type_references,
            path,
            &context.base_dir,
        )?;
        self.resolve_go_type_definitions(&mut result.type_references, path, &context.base_dir)?;

        Ok(result)
    }

    fn can_handle_extension(&self, extension: &str) -> bool {
        extension == "go"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["go"]
    }
}

impl GoAnalyzer {
    fn merge_duplicate_imports(&self, result: &mut AnalysisResult) {
        let mut merged: Vec<Import> = Vec::new();
        let mut index_by_module: HashMap<String, usize> = HashMap::new();

        for import in result.imports.drain(..) {
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

    fn correlate_qualified_symbols_with_imports(&self, result: &mut AnalysisResult) {
        let aliases = go_import_aliases(&result.imports);

        for type_ref in &mut result.type_references {
            if let Some(module) = type_ref.module.as_mut() {
                if let Some(import_path) = aliases.get(module) {
                    *module = import_path.clone();
                }
            }
        }

        for call in &mut result.function_calls {
            if let Some(module) = call.module.as_mut() {
                if let Some(import_path) = aliases.get(module) {
                    *module = import_path.clone();
                }
            }
        }
    }

    fn dedupe_type_references(&self, result: &mut AnalysisResult) {
        let mut deduped: Vec<TypeReference> = Vec::new();
        let mut index_by_name_line: HashMap<(String, usize), usize> = HashMap::new();

        for type_ref in result.type_references.drain(..) {
            let key = (type_ref.name.clone(), type_ref.line);
            if let Some(existing_index) = index_by_name_line.get(&key).copied() {
                let existing = &mut deduped[existing_index];
                if existing.module.is_none() && type_ref.module.is_some() {
                    *existing = type_ref;
                }
            } else {
                index_by_name_line.insert(key, deduped.len());
                deduped.push(type_ref);
            }
        }

        result.type_references = deduped;
    }

    fn resolve_go_type_definitions(
        &self,
        type_refs: &mut [TypeReference],
        current_file: &Path,
        project_root: &Path,
    ) -> Result<(), ContextCreatorError> {
        let resolver = GoModuleResolver;

        for type_ref in type_refs.iter_mut() {
            if type_ref.definition_path.is_some() || type_ref.is_external {
                continue;
            }

            let definition = if let Some(module_path) = type_ref.module.as_deref() {
                match resolver.resolve_import(module_path, current_file, project_root) {
                    Ok(resolved) if resolved.is_external => {
                        type_ref.is_external = true;
                        type_ref.external_package = Some(module_path.to_string());
                        None
                    }
                    Ok(resolved) => {
                        find_go_type_definition_in_package(&resolved.path, &type_ref.name)?
                    }
                    Err(_) => None,
                }
            } else {
                let package_dir = current_file.parent().unwrap_or(project_root);
                find_go_type_definition_in_package(package_dir, &type_ref.name)?
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
}

pub struct GoModuleResolver;

impl ModuleResolver for GoModuleResolver {
    fn resolve_import(
        &self,
        module_path: &str,
        from_file: &Path,
        base_dir: &Path,
    ) -> Result<ResolvedPath, ContextCreatorError> {
        let module_path = clean_go_import_path(module_path);
        validate_module_name(&module_path)?;

        if module_path.starts_with('.') {
            if let Some(from_dir) = from_file.parent() {
                let target = from_dir.join(&module_path);
                if let Some(path) = find_go_package_file(&target) {
                    return resolved_go_path(path, base_dir, false, 0.9);
                }
            }
        }

        if let Some(go_module) = read_go_module_path(base_dir) {
            if module_path == go_module {
                if let Some(path) = find_go_package_file(base_dir) {
                    return resolved_go_path(path, base_dir, false, 0.95);
                }
            } else if let Some(relative_path) = module_path.strip_prefix(&format!("{go_module}/")) {
                let target = base_dir.join(relative_path);
                if let Some(path) = find_go_package_file(&target) {
                    return resolved_go_path(path, base_dir, false, 0.95);
                }
            }
        }

        let project_relative = join_go_import_path(base_dir, &module_path);
        if let Some(path) = find_go_package_file(&project_relative) {
            return resolved_go_path(path, base_dir, false, 0.8);
        }

        if let Some(from_dir) = from_file.parent() {
            let sibling_relative = join_go_import_path(from_dir, &module_path);
            if let Some(path) = find_go_package_file(&sibling_relative) {
                return resolved_go_path(path, base_dir, false, 0.7);
            }
        }

        Ok(ResolvedPath {
            path: base_dir.join("go.mod"),
            is_external: true,
            confidence: if self.is_external_module(&module_path) {
                0.9
            } else {
                0.5
            },
        })
    }

    fn get_file_extensions(&self) -> Vec<&'static str> {
        vec!["go"]
    }

    fn is_external_module(&self, module_path: &str) -> bool {
        let module_path = clean_go_import_path(module_path);

        if module_path.starts_with('.') {
            return false;
        }

        if module_path
            .split('/')
            .next()
            .is_some_and(|part| part.contains('.'))
        {
            return true;
        }

        is_go_standard_library_path(&module_path)
    }
}

fn clean_go_import_path(module_path: &str) -> String {
    module_path
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('`')
        .to_string()
}

fn read_go_module_path(base_dir: &Path) -> Option<String> {
    let go_mod = fs::read_to_string(base_dir.join("go.mod")).ok()?;

    go_mod.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("module ")
            .map(str::trim)
            .filter(|module| !module.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn join_go_import_path(base: &Path, module_path: &str) -> PathBuf {
    module_path
        .split('/')
        .filter(|part| !part.is_empty())
        .fold(base.to_path_buf(), |path, part| path.join(part))
}

fn resolved_go_path(
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

fn find_go_package_file(package_path: &Path) -> Option<PathBuf> {
    if package_path.is_file() && package_path.extension().and_then(|ext| ext.to_str()) == Some("go")
    {
        return Some(package_path.to_path_buf());
    }

    let direct_file = package_path.with_extension("go");
    if direct_file.is_file() && !is_go_test_file(&direct_file) {
        return Some(direct_file);
    }

    if !package_path.is_dir() {
        return None;
    }

    if let Some(package_name) = package_path.file_name().and_then(|name| name.to_str()) {
        let same_name = package_path.join(format!("{package_name}.go"));
        if same_name.is_file() && !is_go_test_file(&same_name) {
            return Some(same_name);
        }
    }

    for preferred in ["doc.go", "types.go"] {
        let candidate = package_path.join(preferred);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let mut go_files = fs::read_dir(package_path)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|ext| ext.to_str()) == Some("go") && !is_go_test_file(path)
        })
        .collect::<Vec<_>>();

    go_files.sort();
    go_files.into_iter().next()
}

fn is_go_test_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with("_test.go"))
}

fn go_import_aliases(
    imports: &[crate::core::semantic::analyzer::Import],
) -> HashMap<String, String> {
    let mut aliases = HashMap::new();

    for import in imports {
        if import.module.is_empty() {
            continue;
        }

        let explicit_alias = import
            .items
            .iter()
            .find(|item| item.as_str() != "_" && item.as_str() != "." && !item.starts_with("as "))
            .cloned();

        let alias = explicit_alias.or_else(|| {
            import
                .module
                .rsplit('/')
                .next()
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned)
        });

        if let Some(alias) = alias {
            aliases.insert(alias, import.module.clone());
        }
    }

    aliases
}

fn find_go_type_definition_in_package(
    package_path: &Path,
    type_name: &str,
) -> Result<Option<PathBuf>, ContextCreatorError> {
    let package_dir = if package_path.is_file() {
        package_path.parent().unwrap_or(package_path)
    } else {
        package_path
    };

    let mut candidates = Vec::new();
    if package_path.is_file() {
        candidates.push(package_path.to_path_buf());
    }

    if package_dir.is_dir() {
        let mut package_files = fs::read_dir(package_dir)
            .map_err(|e| {
                ContextCreatorError::ReadError(format!(
                    "Failed to read Go package directory {}: {e}",
                    package_dir.display()
                ))
            })?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().and_then(|ext| ext.to_str()) == Some("go")
                    && !is_go_test_file(path)
            })
            .collect::<Vec<_>>();
        package_files.sort();

        let mut seen = candidates.iter().cloned().collect::<HashSet<_>>();
        for path in package_files {
            if seen.insert(path.clone()) {
                candidates.push(path);
            }
        }
    }

    for candidate in candidates {
        let content = fs::read_to_string(&candidate).map_err(|e| {
            ContextCreatorError::ReadError(format!(
                "Failed to read Go file {}: {e}",
                candidate.display()
            ))
        })?;

        if go_file_contains_type_definition(&content, type_name)? {
            return Ok(Some(candidate));
        }
    }

    Ok(None)
}

fn go_file_contains_type_definition(
    content: &str,
    type_name: &str,
) -> Result<bool, ContextCreatorError> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_go::language())
        .map_err(|e| ContextCreatorError::ParseError(format!("Failed to set language: {e}")))?;

    let Some(tree) = parser.parse(content, None) else {
        return Ok(false);
    };

    let query_text = r#"
        [
          (type_spec name: (type_identifier) @name)
          (type_alias name: (type_identifier) @name)
        ]
    "#;
    let query = Query::new(tree_sitter_go::language(), query_text).map_err(|e| {
        ContextCreatorError::ParseError(format!("Failed to create Go type query: {e}"))
    })?;
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            if let Ok(captured_text) = capture.node.utf8_text(content.as_bytes()) {
                if captured_text == type_name {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

fn is_go_standard_library_path(module_path: &str) -> bool {
    let first_segment = module_path.split('/').next().unwrap_or(module_path);
    matches!(
        first_segment,
        "archive"
            | "bufio"
            | "bytes"
            | "cmp"
            | "compress"
            | "container"
            | "context"
            | "crypto"
            | "database"
            | "debug"
            | "embed"
            | "encoding"
            | "errors"
            | "expvar"
            | "flag"
            | "fmt"
            | "go"
            | "hash"
            | "html"
            | "image"
            | "index"
            | "io"
            | "iter"
            | "log"
            | "maps"
            | "math"
            | "mime"
            | "net"
            | "os"
            | "path"
            | "plugin"
            | "reflect"
            | "regexp"
            | "runtime"
            | "slices"
            | "sort"
            | "strconv"
            | "strings"
            | "structs"
            | "sync"
            | "syscall"
            | "testing"
            | "text"
            | "time"
            | "unicode"
            | "unsafe"
            | "weak"
    )
}
