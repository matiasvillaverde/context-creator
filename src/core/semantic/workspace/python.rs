use super::manifests;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn local_dependencies(manifest: &Value) -> Vec<(String, String)> {
    let mut dependencies = Vec::new();
    if let Some(poetry) = manifest.pointer("/tool/poetry") {
        let mut groups = vec![poetry.get("dependencies"), poetry.get("dev-dependencies")];
        if let Some(extra) = poetry.get("group").and_then(Value::as_object) {
            groups.extend(extra.values().map(|group| group.get("dependencies")));
        }
        for group in groups.into_iter().flatten().filter_map(Value::as_object) {
            for (name, value) in group {
                if let Some(path) = value.get("path").and_then(Value::as_str) {
                    dependencies.push((name.clone(), path.into()));
                }
            }
        }
    }
    if let Some(sources) = manifest
        .pointer("/tool/uv/sources")
        .and_then(Value::as_object)
    {
        for (name, value) in sources {
            if let Some(path) = value.get("path").and_then(Value::as_str) {
                dependencies.push((name.clone(), path.into()));
            }
        }
    }
    let mut groups = vec![manifest.pointer("/project/dependencies")];
    if let Some(optional) = manifest
        .pointer("/project/optional-dependencies")
        .and_then(Value::as_object)
    {
        groups.extend(optional.values().map(Some));
    }
    for group in groups {
        for dependency in manifests::strings(group) {
            if let Some((name, path)) = dependency.split_once('@') {
                let path = path.split(';').next().unwrap_or("").trim();
                if let Some(path) = path
                    .strip_prefix("file://")
                    .or_else(|| path.strip_prefix("file:"))
                {
                    dependencies.push((name.trim().into(), path.trim().into()));
                }
            }
        }
    }
    dependencies
}

pub(super) fn is_member(directory: &Path, root: &Path) -> bool {
    for filename in ["pyproject.toml", "setup.py"] {
        let Some(manifest) = manifests::read(root, &root.join(filename)) else {
            continue;
        };
        if local_dependencies(&manifest).iter().any(|(_, path)| {
            manifests::confined(root, &root.join(path))
                .is_some_and(|member| directory.starts_with(member))
        }) {
            return true;
        }
        let patterns = manifests::strings(manifest.pointer("/tool/uv/workspace/members"));
        if manifests::members(root, &patterns)
            .iter()
            .any(|member| directory.starts_with(member))
        {
            return true;
        }
    }
    false
}

pub(super) fn resolve(module: &str, source: &Path, root: &Path) -> Option<(PathBuf, f32)> {
    if module.starts_with('.') {
        return None;
    }
    let mut matches = Vec::new();
    for directory in manifests::ancestors(source.parent()?, root) {
        for filename in ["pyproject.toml", "setup.py"] {
            let Some(manifest) = manifests::read(root, &directory.join(filename)) else {
                continue;
            };
            for (name, path) in local_dependencies(&manifest) {
                let Some(package) = manifests::confined(root, &directory.join(path)) else {
                    continue;
                };
                if let Some(path) = module_file(root, &package, &name, module) {
                    matches.push(path);
                }
            }
            let patterns = manifests::strings(manifest.pointer("/tool/uv/workspace/members"));
            for member in manifests::members(directory, &patterns) {
                let Some(package) = manifests::read(root, &member.join("pyproject.toml")) else {
                    continue;
                };
                let Some(name) = package.pointer("/project/name").and_then(Value::as_str) else {
                    continue;
                };
                let declared = manifest
                    .pointer("/tool/uv/sources")
                    .and_then(|sources| sources.get(name))
                    .and_then(|source| source.get("workspace"))
                    .and_then(Value::as_bool)
                    == Some(true);
                if declared {
                    if let Some(path) = module_file(root, &member, name, module) {
                        matches.push(path);
                    }
                }
            }
        }
    }
    matches.sort();
    matches.dedup();
    if matches.len() == 1 {
        matches.pop().map(|path| (path, 0.85))
    } else {
        None
    }
}

fn module_file(root: &Path, package: &Path, name: &str, module: &str) -> Option<PathBuf> {
    let (prefix, tail) = module.split_once('.').unwrap_or((module, ""));
    let module = module.replace('.', "/");
    let mut bases = vec![package.join("src"), package.to_path_buf()];
    for filename in ["pyproject.toml", "setup.py"] {
        let Some(manifest) = manifests::read(root, &package.join(filename)) else {
            continue;
        };
        for item in manifest
            .pointer("/tool/poetry/packages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(from) = item.get("from").and_then(Value::as_str) {
                bases.push(package.join(from));
            }
        }
        for from in manifests::strings(manifest.pointer("/tool/setuptools/packages/find/where")) {
            bases.push(package.join(from));
        }
        if let Some(from) = manifest
            .pointer("/tool/setuptools/package-dir")
            .and_then(|mapping| mapping.get(""))
            .and_then(Value::as_str)
        {
            bases.push(package.join(from));
        }
    }
    let mut candidates = bases
        .into_iter()
        .map(|base| base.join(&module))
        .collect::<Vec<_>>();
    if name.replace(['-', '.'], "_") == prefix && package.join("__init__.py").is_file() {
        candidates.push(package.join(tail.replace('.', "/")));
    }
    for candidate in candidates {
        for path in [
            candidate.with_extension("py"),
            candidate.with_extension("pyi"),
            candidate.join("__init__.py"),
        ] {
            if path.is_file() {
                if let Some(path) = manifests::confined(root, &path) {
                    return Some(path);
                }
            }
        }
    }
    None
}
