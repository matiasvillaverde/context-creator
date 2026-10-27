use super::manifests;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(super) fn is_member(directory: &Path, root: &Path) -> bool {
    let Some(document) = manifests::read(root, &root.join("Cargo.toml")) else {
        return false;
    };
    let Some(workspace) = document.get("workspace") else {
        return false;
    };
    let patterns = manifests::strings(workspace.get("members"));
    let excluded = manifests::strings(workspace.get("exclude"));
    let members = manifests::members(root, &patterns);
    members
        .iter()
        .filter(|member| member.join("Cargo.toml").is_file())
        .any(|member| directory.starts_with(member))
        && !manifests::members(root, &excluded)
            .iter()
            .any(|member| directory.starts_with(member))
}

pub(super) fn resolve(module: &str, source: &Path, root: &Path) -> Option<(PathBuf, f32)> {
    if module.starts_with("self::") || module.starts_with("super::") {
        return None;
    }
    let (name, tail) = module.split_once("::").unwrap_or((module, ""));
    let (package_root, package) =
        manifests::ancestors(source.parent()?, root).find_map(|directory| {
            manifests::read(root, &directory.join("Cargo.toml"))
                .filter(|manifest| manifest.get("package").is_some())
                .map(|manifest| (directory, manifest))
        })?;
    let own_name = package
        .pointer("/lib/name")
        .or_else(|| package.pointer("/package/name"))?
        .as_str()?
        .replace('-', "_");
    if matches!(name, "std" | "core" | "alloc" | "proc_macro" | "test")
        && dependency(&package, name).is_none()
    {
        return None;
    }
    if name == "crate" || name == own_name {
        return module_file(root, package_root, &package, tail).map(|path| (path, 0.95));
    }
    let dependency = dependency(&package, name)?;
    let mut location = package_root;
    let mut value = dependency;
    let inherited;
    if dependency.get("workspace").and_then(Value::as_bool) == Some(true) {
        let (directory, workspace) =
            manifests::ancestors(package_root, root).find_map(|directory| {
                manifests::read(root, &directory.join("Cargo.toml"))
                    .filter(|manifest| manifest.pointer("/workspace/dependencies").is_some())
                    .map(|manifest| (directory, manifest))
            })?;
        inherited = workspace;
        value = inherited
            .pointer("/workspace/dependencies")?
            .as_object()?
            .iter()
            .find(|(key, _)| key.replace('-', "_") == name)
            .map(|(_, value)| value)?;
        location = directory;
    }
    let path = value.get("path")?.as_str()?;
    let target = manifests::confined(root, &location.join(path))?;
    let manifest = manifests::read(root, &target.join("Cargo.toml"))?;
    module_file(root, &target, &manifest, tail).map(|path| (path, 0.95))
}

pub(crate) fn dependency<'a>(manifest: &'a Value, name: &str) -> Option<&'a Value> {
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(value) = manifest
            .get(section)
            .and_then(Value::as_object)
            .and_then(|items| {
                items
                    .iter()
                    .find(|(key, _)| key.replace('-', "_") == name)
                    .map(|(_, value)| value)
            })
        {
            return Some(value);
        }
    }
    for target in manifest
        .get("target")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|targets| targets.values())
    {
        if let Some(value) = dependency(target, name) {
            return Some(value);
        }
    }
    None
}

fn module_file(root: &Path, package: &Path, manifest: &Value, tail: &str) -> Option<PathBuf> {
    let entry = package.join(
        manifest
            .pointer("/lib/path")
            .and_then(Value::as_str)
            .unwrap_or("src/lib.rs"),
    );
    let entry = manifests::confined(root, &entry)?;
    let parts = tail
        .split("::")
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    for count in (1..=parts.len()).rev() {
        let relative = parts[..count].join("/");
        let candidate = entry.parent()?.join(relative);
        for path in [candidate.with_extension("rs"), candidate.join("mod.rs")] {
            if path.is_file() {
                if let Some(path) = manifests::confined(root, &path) {
                    return Some(path);
                }
            }
        }
    }
    Some(entry)
}
