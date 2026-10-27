//! Local package bindings from repository manifests; no package manager is executed.

mod node;
mod python;
mod rust;

use super::{analyzer::AnalysisResult, manifests, resolver::ResolvedPath};
use std::{
    collections::HashMap,
    collections::HashSet,
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

struct Declarations {
    names: HashSet<String>,
    imports: OnceLock<Vec<super::analyzer::Import>>,
}
type DeclarationCache = HashMap<PathBuf, (u64, Arc<Declarations>)>;
static DECLARATIONS: OnceLock<Mutex<DeclarationCache>> = OnceLock::new();

fn declarations(path: &Path, source: &str) -> Option<Arc<Declarations>> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    let hash = hasher.finish();
    let cache = DECLARATIONS.get_or_init(Default::default);
    if let Some((old, declarations)) = cache.lock().ok()?.get(path) {
        if *old == hash {
            return Some(declarations.clone());
        }
    }
    let names = super::symbols::extract_symbols(path, source)
        .ok()?
        .into_iter()
        .map(|symbol| symbol.name)
        .collect::<HashSet<_>>();
    let declarations = Arc::new(Declarations {
        names,
        imports: OnceLock::new(),
    });
    if declarations.names.len() <= 4096 {
        let mut cache = cache.lock().ok()?;
        if cache.len() >= 32 {
            if let Some(oldest) = cache.keys().next().cloned() {
                cache.remove(&oldest);
            }
        }
        cache.insert(path.to_path_buf(), (hash, declarations.clone()));
    }
    Some(declarations)
}

pub(crate) fn resolve_import(module: &str, source: &Path, root: &Path) -> Option<ResolvedPath> {
    if !matches!(
        source.extension()?.to_str()?,
        "rs" | "py" | "pyi" | "pyw" | "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts"
    ) {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let source = manifests::confined(&root, source)?;
    let (path, confidence) = match source.extension()?.to_str()? {
        "rs" => rust::resolve(module, &source, &root),
        "py" | "pyi" | "pyw" => python::resolve(module, &source, &root),
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts" => {
            node::resolve(module, &source, &root)
        }
        _ => None,
    }?;
    Some(ResolvedPath {
        path,
        is_external: false,
        confidence,
    })
}

pub(crate) fn root_for(source: &Path) -> Option<PathBuf> {
    let source = source.canonicalize().ok()?;
    let directory = if source.is_file() {
        source.parent()?
    } else {
        &source
    };
    for ancestor in directory.ancestors() {
        if rust::is_member(directory, ancestor)
            || node::is_member(directory, ancestor)
            || python::is_member(directory, ancestor)
        {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

pub(crate) fn correlate_imports(result: &mut AnalysisResult, source: &Path, root: &Path) {
    for reference in &mut result.type_references {
        for import in &result.imports {
            for item in &import.items {
                let binding = if let Some(alias) = item.strip_prefix("as ") {
                    import.module.rsplit("::").next().map(|name| (name, alias))
                } else {
                    item.split_once(" as ")
                };
                if let Some((name, alias)) = binding {
                    if reference.name == alias
                        && name != "*"
                        && name.chars().next().is_some_and(char::is_uppercase)
                    {
                        reference.module = Some(import.module.clone());
                        if let Some(definition) =
                            type_definition(name, &import.module, source, root)
                        {
                            reference.definition_path = definition;
                        }
                    } else if reference.module.as_deref() == Some(alias) {
                        reference.module = Some(import.module.clone());
                    }
                }
            }
        }
        if reference.module.is_none() {
            let mut modules = result
                .imports
                .iter()
                .filter(|import| import.items.iter().any(|item| item == &reference.name))
                .map(|import| &import.module);
            if let Some(module) = modules.next() {
                if modules.all(|other| other == module) {
                    reference.module = Some(module.clone());
                }
            }
        }
    }
}

/// Some(None) is a known local binding without a definition; do not search unrelated packages.
pub(crate) fn type_definition(
    name: &str,
    module: &str,
    source: &Path,
    root: &Path,
) -> Option<Option<PathBuf>> {
    let binding = resolve_import(module, source, root)?;
    Some(find_definition(
        name,
        &binding.path,
        root,
        &mut HashSet::new(),
        0,
    ))
}

fn find_definition(
    name: &str,
    path: &Path,
    root: &Path,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> Option<PathBuf> {
    if depth >= 10 || !visited.insert(path.to_path_buf()) {
        return None;
    }
    let path = manifests::confined(root, path)?;
    let mut source = String::new();
    std::fs::File::open(&path)
        .ok()?
        .take(2 * 1024 * 1024 + 1)
        .read_to_string(&mut source)
        .ok()?;
    if source.len() > 2 * 1024 * 1024 {
        return None;
    }
    let declarations = declarations(&path, &source)?;
    if declarations.names.contains(name) {
        return Some(path);
    }
    // Follow explicit re-exports/imports only, retaining file-cycle and depth guards.
    let language = match path.extension()?.to_str()? {
        "rs" => "rust",
        "py" | "pyi" => "python",
        "ts" | "tsx" | "mts" | "cts" => "typescript",
        _ => "javascript",
    };
    let imports = declarations.imports.get_or_init(|| {
        let extract = || {
            let grammar = super::grammar_for_file(&path)?;
            let query = super::query_engine::QueryEngine::new(grammar.clone(), language).ok()?;
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(&grammar).ok()?;
            query
                .analyze_with_parser(&mut parser, &source)
                .ok()
                .map(|analysis| analysis.imports)
        };
        extract().unwrap_or_default()
    });
    for import in imports {
        let aliased = import.items.iter().find_map(|item| {
            item.split_once(" as ")
                .filter(|(_, alias)| *alias == name)
                .map(|(actual, _)| actual)
                .or_else(|| {
                    (item.strip_prefix("as ") == Some(name))
                        .then(|| import.module.rsplit("::").next())
                        .flatten()
                })
        });
        if aliased.is_none()
            && !import.items.iter().any(|item| item == name || item == "*")
            && !import.module.ends_with(&format!("::{name}"))
        {
            continue;
        }
        let resolver = super::get_resolver_for_file(&path).ok()??;
        let resolved = resolver.resolve_import(&import.module, &path, root).ok()?;
        if !resolved.is_external {
            if let Some(path) = find_definition(
                aliased.unwrap_or(name),
                &resolved.path,
                root,
                visited,
                depth + 1,
            ) {
                return Some(path);
            }
        }
    }
    None
}

pub(crate) fn node_version_matches(requirement: &str, version: &str) -> bool {
    node::satisfies(requirement, version)
}
