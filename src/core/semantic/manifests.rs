//! Bounded, content-checked manifest parsing shared by local package resolution.

use super::parsing::ParserExt;
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::File,
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

const MAX_BYTES: u64 = 1024 * 1024;
type Documents = HashMap<PathBuf, (u64, Option<Arc<Value>>, usize)>;
static DOCUMENTS: OnceLock<Mutex<Documents>> = OnceLock::new();

pub(crate) fn confined(root: &Path, path: &Path) -> Option<PathBuf> {
    let root = root.canonicalize().ok()?;
    let canonical = super::path_validator::validate_import_path(&root, path).ok()?;
    let mut prefix = PathBuf::new();
    for part in path.components() {
        prefix.push(part);
        if prefix.starts_with(&root) && prefix != root {
            if std::fs::symlink_metadata(&prefix)
                .ok()?
                .file_type()
                .is_symlink()
            {
                return None;
            }
            if prefix
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with('.'))
            {
                return None;
            }
        }
    }
    Some(canonical)
}

pub(crate) fn read(root: &Path, path: &Path) -> Option<Arc<Value>> {
    let path = confined(root, path)?;
    let handle = File::open(&path).ok()?;
    if !handle.metadata().ok()?.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    handle.take(MAX_BYTES + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 > MAX_BYTES {
        return None;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    let documents = DOCUMENTS.get_or_init(Default::default);
    let mut documents = documents.lock().ok()?;
    if let Some((old, document, _)) = documents.get(&path) {
        if *old == hash {
            return document.clone();
        }
    }
    let source = std::str::from_utf8(&bytes).ok()?;
    let document = match path.extension()?.to_str()? {
        "toml" | "lock" => toml::from_str::<toml::Value>(source)
            .ok()
            .and_then(|document| serde_json::to_value(document).ok()),
        "json" => serde_json::from_str(source).ok(),
        "py" => setup_manifest(source),
        _ => Some(Value::String(source.into())),
    }
    .map(Arc::new);
    let weight = bytes.len().saturating_mul(4);
    documents.remove(&path);
    while documents.len() >= 128
        || documents.values().map(|entry| entry.2).sum::<usize>() + weight > 32 * 1024 * 1024
    {
        let oldest = documents.keys().next().cloned()?;
        documents.remove(&oldest);
    }
    documents.insert(path, (hash, document.clone(), weight));
    document
}

pub(crate) fn ancestors<'a>(from: &'a Path, root: &'a Path) -> impl Iterator<Item = &'a Path> {
    from.ancestors()
        .take_while(move |directory| directory.starts_with(root))
}

pub(crate) fn strings(value: Option<&Value>) -> Vec<&str> {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

/// Expand declared member patterns component by component, without walking source trees.
pub(crate) fn members(root: &Path, patterns: &[&str]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for pattern in patterns.iter().filter(|pattern| !pattern.starts_with('!')) {
        if pattern.contains("..") || Path::new(pattern).is_absolute() || pattern.contains('\\') {
            continue;
        }
        let mut current = vec![root.to_path_buf()];
        for part in pattern
            .split('/')
            .filter(|part| !part.is_empty() && *part != ".")
        {
            let mut next = Vec::new();
            let Ok(matcher) = glob::Pattern::new(part) else {
                return Vec::new();
            };
            for directory in current {
                if part == "**" {
                    let mut pending = vec![(directory, 0)];
                    while let Some((directory, depth)) = pending.pop() {
                        next.push(directory.clone());
                        if next.len() > 2048 || depth > 8 {
                            return Vec::new();
                        }
                        let Ok(entries) = std::fs::read_dir(&directory) else {
                            continue;
                        };
                        let mut entries = entries.flatten().take(2049).collect::<Vec<_>>();
                        if entries.len() > 2048 {
                            return Vec::new();
                        }
                        entries.sort_by_key(|entry| entry.file_name());
                        for entry in entries {
                            let name = entry.file_name();
                            let name = name.to_string_lossy();
                            if name.starts_with('.')
                                || matches!(name.as_ref(), "node_modules" | "target" | "vendor")
                            {
                                continue;
                            }
                            if entry.file_type().is_ok_and(|kind| kind.is_dir())
                                && confined(root, &entry.path()).is_some()
                            {
                                pending.push((entry.path(), depth + 1));
                            }
                        }
                        if pending.len() + next.len() > 2048 {
                            return Vec::new();
                        }
                    }
                    continue;
                }
                if !part.contains(['*', '?', '[']) {
                    let path = directory.join(part);
                    if path.is_dir() && confined(root, &path).is_some() {
                        next.push(path);
                    }
                    continue;
                }
                if let Ok(entries) = std::fs::read_dir(&directory) {
                    let mut entries = entries.flatten().take(2049).collect::<Vec<_>>();
                    if entries.len() > 2048 {
                        return Vec::new();
                    }
                    entries.sort_by_key(|entry| entry.file_name());
                    for entry in entries {
                        let path = entry.path();
                        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                            continue;
                        }
                        let name = entry.file_name();
                        let name = name.to_string_lossy();
                        if name.starts_with('.')
                            || matches!(name.as_ref(), "node_modules" | "target" | "vendor")
                        {
                            continue;
                        }
                        if matcher.matches(&name) && confined(root, &path).is_some() {
                            next.push(path);
                        }
                        if next.len() > 256 {
                            return Vec::new();
                        }
                    }
                }
            }
            next.sort();
            next.dedup();
            if next.len() > 2048 {
                return Vec::new();
            }
            current = next;
        }
        found.extend(current.into_iter().filter(|directory| {
            ["Cargo.toml", "package.json", "pyproject.toml", "setup.py"]
                .iter()
                .any(|name| directory.join(name).is_file())
        }));
        if found.len() > 256 {
            return Vec::new();
        }
    }
    found.retain(|path| {
        let Ok(relative) = path.strip_prefix(root) else {
            return false;
        };
        !patterns
            .iter()
            .filter_map(|p| p.strip_prefix('!'))
            .any(|pattern| {
                glob::Pattern::new(pattern).is_ok_and(|pattern| pattern.matches_path(relative))
            })
    });
    found.sort();
    found.dedup();
    found.truncate(256);
    found
}

fn setup_manifest(source: &str) -> Option<Value> {
    fn literal(node: tree_sitter::Node<'_>, source: &str, depth: usize) -> Option<Value> {
        if depth > 8 {
            return None;
        }
        match node.kind() {
            "string" => {
                let text = node.utf8_text(source.as_bytes()).ok()?;
                let quote = text.chars().next()?;
                if !matches!(quote, '\'' | '"') || !text.ends_with(quote) || text.contains('\\') {
                    return None;
                }
                Some(Value::String(text[1..text.len() - 1].into()))
            }
            "list" | "tuple" => {
                let mut cursor = node.walk();
                node.named_children(&mut cursor)
                    .map(|child| literal(child, source, depth + 1))
                    .collect::<Option<Vec<_>>>()
                    .map(Value::Array)
            }
            "dictionary" => {
                let mut values = serde_json::Map::new();
                let mut cursor = node.walk();
                for pair in node.named_children(&mut cursor) {
                    let key = literal(pair.child_by_field_name("key")?, source, depth + 1)?
                        .as_str()?
                        .to_string();
                    values.insert(
                        key,
                        literal(pair.child_by_field_name("value")?, source, depth + 1)?,
                    );
                }
                Some(Value::Object(values))
            }
            _ => None,
        }
    }
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .ok()?;
    let tree = parser.parse_bounded(source, None)?;
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.kind() == "call" {
            let function = node
                .child_by_field_name("function")?
                .utf8_text(source.as_bytes())
                .ok()?;
            if function == "setup" || function == "setuptools.setup" {
                let mut fields = serde_json::Map::new();
                let arguments = node.child_by_field_name("arguments")?;
                let mut cursor = arguments.walk();
                for argument in arguments.named_children(&mut cursor) {
                    if argument.kind() != "keyword_argument" {
                        continue;
                    }
                    let Some(name) = argument
                        .child_by_field_name("name")
                        .and_then(|name| name.utf8_text(source.as_bytes()).ok())
                    else {
                        continue;
                    };
                    if let Some(value) = argument
                        .child_by_field_name("value")
                        .and_then(|value| literal(value, source, 0))
                    {
                        fields.insert(name.into(), value);
                    }
                }
                return Some(serde_json::json!({
                    "project":{"name":fields.get("name"),"version":fields.get("version"),
                        "dependencies":fields.get("install_requires"),"optional-dependencies":fields.get("extras_require")},
                    "tool":{"setuptools":{"package-dir":fields.get("package_dir")}}
                }));
            }
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchanged_documents_reuse_parsing_and_content_edits_invalidate_it() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("package.json");
        std::fs::write(&file, r#"{"name":"one"}"#).unwrap();
        let first = read(dir.path(), &file).unwrap();
        assert!(Arc::ptr_eq(&first, &read(dir.path(), &file).unwrap()));
        std::fs::write(&file, r#"{"name":"two"}"#).unwrap();
        let edited = read(dir.path(), &file).unwrap();
        assert!(!Arc::ptr_eq(&first, &edited));
        assert_eq!(edited["name"], "two");
    }
}
