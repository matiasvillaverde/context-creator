use super::*;
use serde_json::Value;
pub struct PythonPackageResolver;
fn normalized(name: &str) -> String {
    name.to_ascii_lowercase().replace(['_', '.'], "-")
}
fn pep(requirement: &str, name: &str) -> Option<String> {
    let end = requirement
        .find(|c: char| !c.is_ascii_alphanumeric() && !"._-".contains(c))
        .unwrap_or(requirement.len());
    if normalized(&requirement[..end]) != normalized(name) {
        return None;
    }
    let mut tail = requirement[end..].trim();
    if tail.starts_with('[') {
        tail = tail.split_once(']')?.1.trim();
    }
    Some(
        tail.split(';')
            .next()?
            .trim()
            .trim_matches(['(', ')'])
            .to_owned(),
    )
}
impl PackageVersionResolver for PythonPackageResolver {
    fn resolve_version(
        &self,
        module: &str,
        source: &Path,
        root: &Path,
        include_dev: bool,
    ) -> Option<String> {
        let root = root.canonicalize().ok()?;
        let source = manifests::confined(&root, source)?;
        let name = module.split('.').next()?;
        // Import names can differ from distribution names; do not guess their mapping.
        for directory in manifests::ancestors(source.parent()?, &root) {
            let mut project_boundary = false;
            for file in ["pyproject.toml", "setup.py"] {
                let Some(document) = manifests::read(&root, &directory.join(file)) else {
                    continue;
                };
                let mut declared = manifests::strings(document.pointer("/project/dependencies"))
                    .into_iter()
                    .find_map(|v| pep(v, name))
                    .map(|v| (name.to_owned(), v, false));
                if declared.is_none() {
                    declared = document
                        .pointer("/project/optional-dependencies")
                        .and_then(Value::as_object)
                        .into_iter()
                        .flat_map(|groups| groups.values())
                        .flat_map(|v| manifests::strings(Some(v)))
                        .find_map(|v| pep(v, name))
                        .map(|v| (name.to_owned(), v, false));
                }
                let mut sections = vec![(document.pointer("/tool/poetry/dependencies"), false)];
                if include_dev {
                    sections.push((document.pointer("/tool/poetry/dev-dependencies"), true));
                    sections.extend(
                        document
                            .pointer("/tool/poetry/group")
                            .and_then(Value::as_object)
                            .into_iter()
                            .flat_map(|groups| groups.values())
                            .map(|group| (group.get("dependencies"), true)),
                    );
                }
                if declared.is_none() {
                    declared = sections.into_iter().find_map(|(section, dev)| {
                        let (key, value) = section?
                            .as_object()?
                            .iter()
                            .find(|(key, _)| normalized(key) == normalized(name))?;
                        if value.get("path").is_some() {
                            return None;
                        }
                        let version = value
                            .as_str()
                            .or_else(|| value.get("version").and_then(Value::as_str))
                            .unwrap_or("");
                        Some((key.clone(), version.to_owned(), dev))
                    });
                }
                if let Some((name, declared, dev)) = declared {
                    return describe(&name, Some(&declared), None, dev);
                }
                project_boundary |= document.get("project").is_some()
                    || document.pointer("/tool/poetry/name").is_some();
            }
            for (file, dev) in [("requirements.txt", false), ("requirements-dev.txt", true)] {
                if dev && !include_dev {
                    continue;
                }
                let Some(document) = manifests::read(&root, &directory.join(file)) else {
                    continue;
                };
                if let Some(declared) = document
                    .as_str()?
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.starts_with(['#', '-']))
                    .find_map(|v| pep(v, name))
                {
                    return describe(name, Some(&declared), None, dev);
                }
            }
            if project_boundary {
                return None;
            }
        }
        None
    }
}
