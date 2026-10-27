use super::*;
use serde_json::Value;
pub struct CargoPackageResolver;
impl PackageVersionResolver for CargoPackageResolver {
    fn resolve_version(
        &self,
        module: &str,
        source: &Path,
        root: &Path,
        include_dev: bool,
    ) -> Option<String> {
        let root = root.canonicalize().ok()?;
        let source = manifests::confined(&root, source)?;
        let name = module.split("::").next()?;
        let (directory, document) =
            manifests::ancestors(source.parent()?, &root).find_map(|dir| {
                manifests::read(&root, &dir.join("Cargo.toml"))
                    .filter(|v| v.get("package").is_some())
                    .map(|v| (dir, v))
            })?;
        let (key, mut dependency, dev) = dependency(&document, name, include_dev)?;
        let inherited;
        if dependency.get("workspace").and_then(Value::as_bool) == Some(true) {
            inherited = manifests::ancestors(directory, &root).find_map(|dir| {
                manifests::read(&root, &dir.join("Cargo.toml")).filter(|v| {
                    v.pointer("/workspace/dependencies")
                        .and_then(|v| v.get(key))
                        .is_some()
                })
            })?;
            dependency = inherited.pointer("/workspace/dependencies")?.get(key)?;
        }
        // Local packages are described by source relationships, not external versions.
        if dependency.get("path").is_some() {
            return None;
        }
        let package = dependency
            .get("package")
            .and_then(Value::as_str)
            .unwrap_or(key);
        let declared = dependency
            .as_str()
            .or_else(|| dependency.get("version").and_then(Value::as_str));
        let lock = manifests::ancestors(directory, &root)
            .find_map(|dir| manifests::read(&root, &dir.join("Cargo.lock")));
        let locked = lock.as_ref().and_then(|lock| {
            let requirement = semver::VersionReq::parse(declared?).ok()?;
            let mut candidates = lock
                .get("package")?
                .as_array()?
                .iter()
                .filter(|p| p.get("name").and_then(Value::as_str) == Some(package))
                .filter_map(|p| p.get("version")?.as_str())
                .filter(|v| {
                    semver::Version::parse(v)
                        .ok()
                        .is_some_and(|v| requirement.matches(&v))
                });
            let first = candidates.next()?;
            candidates.all(|other| other == first).then_some(first)
        });
        describe(package, declared, locked, dev)
    }
}
fn dependency<'a>(
    document: &'a Value,
    name: &str,
    include_dev: bool,
) -> Option<(&'a str, &'a Value, bool)> {
    for (section, dev) in [
        ("dependencies", false),
        ("build-dependencies", false),
        ("dev-dependencies", true),
    ] {
        if dev && !include_dev {
            continue;
        }
        if let Some((key, value)) = document
            .get(section)
            .and_then(Value::as_object)
            .and_then(|deps| deps.iter().find(|(key, _)| key.replace('-', "_") == name))
        {
            return Some((key, value, dev));
        }
    }
    document
        .get("target")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|targets| targets.values())
        .find_map(|target| dependency(target, name, include_dev))
}
