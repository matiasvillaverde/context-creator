use super::manifests;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn patterns(root: &Path, boundary: &Path) -> Vec<String> {
    let mut patterns = Vec::new();
    if let Some(manifest) = manifests::read(boundary, &root.join("package.json")) {
        let workspaces = manifest.get("workspaces");
        patterns.extend(
            manifests::strings(
                workspaces
                    .and_then(|value| value.get("packages"))
                    .or(workspaces),
            )
            .into_iter()
            .map(str::to_owned),
        );
    }
    if let Some(document) = manifests::read(boundary, &root.join("pnpm-workspace.yaml")) {
        if let Some(text) = document.as_str() {
            let mut in_packages = false;
            for line in text.lines() {
                if let Some(value) = line
                    .trim()
                    .strip_prefix("packages:")
                    .map(str::trim)
                    .filter(|value| value.starts_with('['))
                {
                    if let Ok(document) =
                        toml::from_str::<toml::Value>(&format!("packages = {value}"))
                    {
                        if let Some(values) =
                            document.get("packages").and_then(toml::Value::as_array)
                        {
                            patterns.extend(
                                values
                                    .iter()
                                    .filter_map(toml::Value::as_str)
                                    .map(str::to_owned),
                            );
                        }
                    }
                    break;
                }
                if line.trim() == "packages:" {
                    in_packages = true;
                    continue;
                }
                if !in_packages || line.trim().is_empty() || line.trim_start().starts_with('#') {
                    continue;
                }
                if !line.starts_with(' ') {
                    break;
                }
                let Some(pattern) = line.trim().strip_prefix("- ") else {
                    break;
                };
                let pattern = pattern.split(" #").next().unwrap_or("").trim();
                let pattern = if pattern.starts_with('"') {
                    serde_json::from_str::<String>(pattern).ok()
                } else {
                    Some(pattern.trim_matches('\'').replace("''", "'"))
                };
                if let Some(pattern) = pattern {
                    patterns.push(pattern);
                }
            }
        }
    }
    patterns
}

pub(super) fn is_member(directory: &Path, root: &Path) -> bool {
    let patterns = patterns(root, root);
    manifests::members(
        root,
        &patterns.iter().map(String::as_str).collect::<Vec<_>>(),
    )
    .iter()
    .filter(|member| member.join("package.json").is_file())
    .any(|member| directory.starts_with(member))
}

pub(super) fn resolve(module: &str, source: &Path, root: &Path) -> Option<(PathBuf, f32)> {
    if module.starts_with('.') || module.starts_with('/') {
        return None;
    }
    let parts = module.split('/').collect::<Vec<_>>();
    let count = if module.starts_with('@') { 2 } else { 1 };
    if parts.len() < count {
        return None;
    }
    let name = parts[..count].join("/");
    let subpath = parts[count..].join("/");
    let (directory, manifest) =
        manifests::ancestors(source.parent()?, root).find_map(|directory| {
            manifests::read(root, &directory.join("package.json"))
                .map(|manifest| (directory, manifest))
        })?;
    let specification = [
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
    ]
    .into_iter()
    .find_map(|section| {
        manifest
            .get(section)
            .and_then(|dependencies| dependencies.get(&name))
            .and_then(Value::as_str)
    })?;
    let typescript = matches!(source.extension()?.to_str()?, "ts" | "tsx" | "mts" | "cts");
    if let Some(relative) = specification
        .strip_prefix("file:")
        .or_else(|| specification.strip_prefix("link:"))
    {
        let package = manifests::confined(root, &directory.join(relative))?;
        return entry(root, &package, &subpath, typescript);
    }
    let mut candidates = Vec::new();
    let mut named_members = std::collections::BTreeSet::new();
    for workspace in manifests::ancestors(directory, root) {
        let patterns = patterns(workspace, root);
        let members = manifests::members(
            workspace,
            &patterns.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        if directory != workspace && !members.iter().any(|member| member == directory) {
            continue;
        }
        for member in members {
            let Some(package) = manifests::read(root, &member.join("package.json")) else {
                continue;
            };
            if package.get("name").and_then(Value::as_str) == Some(&name) {
                named_members.insert(member.clone());
                let requested = specification
                    .strip_prefix("workspace:")
                    .unwrap_or(specification);
                let matches = (specification.starts_with("workspace:")
                    && matches!(requested, "*" | "^" | "~"))
                    || package
                        .get("version")
                        .and_then(Value::as_str)
                        .is_some_and(|version| satisfies(requested, version));
                if matches {
                    if let Some(entry) = entry(root, &member, &subpath, typescript) {
                        candidates.push(entry);
                    }
                }
            }
        }
    }
    candidates.sort_by(|a, b| a.0.cmp(&b.0));
    candidates.dedup_by(|a, b| a.0 == b.0);
    if candidates.len() != 1 || named_members.len() != 1 {
        return None;
    }
    let confidence: f32 = if specification.starts_with("workspace:") {
        0.95
    } else {
        0.8
    };
    candidates
        .pop()
        .map(|(path, entry_confidence)| (path, confidence.min(entry_confidence)))
}

fn satisfies(requirement: &str, version: &str) -> bool {
    let Ok(version) = semver::Version::parse(version.trim_start_matches('v')) else {
        return false;
    };
    requirement.split("||").any(|requirement| {
        let requirement = requirement.trim().trim_start_matches('v');
        if let Ok(exact) = semver::Version::parse(requirement) {
            return exact == version;
        }
        let requirement = if requirement.split('.').count() == 2
            && requirement.chars().all(|c| c.is_ascii_digit() || c == '.')
        {
            format!("{requirement}.*")
        } else {
            requirement.split_whitespace().collect::<Vec<_>>().join(",")
        };
        semver::VersionReq::parse(&requirement).is_ok_and(|range| range.matches(&version))
    })
}

fn exported(value: &Value, typescript: bool) -> Option<&str> {
    if let Some(path) = value.as_str() {
        return Some(path);
    }
    let conditions = if typescript {
        ["types", "import", "require", "default"]
    } else {
        ["import", "require", "default", "types"]
    };
    for condition in conditions {
        if let Some(path) = value
            .get(condition)
            .and_then(|value| exported(value, typescript))
        {
            return Some(path);
        }
    }
    None
}

fn entry(root: &Path, package: &Path, subpath: &str, typescript: bool) -> Option<(PathBuf, f32)> {
    let manifest = manifests::read(root, &package.join("package.json"))?;
    let key = if subpath.is_empty() {
        ".".to_string()
    } else {
        format!("./{subpath}")
    };
    let mut entries = Vec::new();
    if let Some(exports) = manifest.get("exports") {
        let path = exports
            .get(&key)
            .or_else(|| subpath.is_empty().then_some(exports))
            .and_then(|value| exported(value, typescript))?;
        entries.push((package.join(path), 0.95));
    } else if !subpath.is_empty() {
        entries.push((package.join(subpath), 0.95));
    } else {
        let fields = if typescript {
            ["types", "typings", "source", "module", "main"]
        } else {
            ["source", "module", "main", "types", "typings"]
        };
        entries.extend(
            fields
                .into_iter()
                .filter_map(|field| manifest.get(field).and_then(Value::as_str))
                .map(|path| (package.join(path), 0.95)),
        );
        entries.push((package.join("index"), 0.85));
        entries.push((package.join("src/index"), 0.8));
    }
    for (candidate, confidence) in entries {
        for path in std::iter::once(candidate.clone()).chain(
            ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"]
                .into_iter()
                .map(|ext| candidate.with_extension(ext)),
        ) {
            if path.is_file() {
                if let Some(path) = manifests::confined(root, &path) {
                    return Some((path, confidence));
                }
            }
        }
        if candidate.is_dir() {
            for ext in ["ts", "tsx", "js", "jsx"] {
                if let Some(path) =
                    manifests::confined(root, &candidate.join(format!("index.{ext}")))
                {
                    return Some((path, confidence));
                }
            }
        }
    }
    None
}
