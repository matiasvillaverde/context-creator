use super::*;
use serde_json::Value;
pub struct NodePackageResolver;
impl PackageVersionResolver for NodePackageResolver {
    fn resolve_version(
        &self,
        module: &str,
        source: &Path,
        root: &Path,
        include_dev: bool,
    ) -> Option<String> {
        let root = root.canonicalize().ok()?;
        let source = manifests::confined(&root, source)?;
        if module.starts_with(['.', '/']) {
            return None;
        }
        let name = module
            .split('/')
            .take(if module.starts_with('@') { 2 } else { 1 })
            .collect::<Vec<_>>()
            .join("/");
        let directory = manifests::ancestors(source.parent()?, &root)
            .find(|dir| dir.join("package.json").exists())?;
        let document = manifests::read(&root, &directory.join("package.json"))?;
        let (declared, dev) = [
            ("dependencies", false),
            ("optionalDependencies", false),
            ("peerDependencies", false),
            ("devDependencies", true),
        ]
        .into_iter()
        .filter(|(_, dev)| include_dev || !dev)
        .find_map(|(key, dev)| document.get(key)?.get(&name)?.as_str().map(|v| (v, dev)))?;
        if ["file:", "link:", "workspace:"]
            .iter()
            .any(|prefix| declared.starts_with(prefix))
        {
            return None;
        }
        let (package, declared) = if let Some(alias) = declared.strip_prefix("npm:") {
            alias.rsplit_once('@')?
        } else {
            (name.as_str(), declared)
        };
        let locked = manifests::ancestors(directory, &root).find_map(|dir| {
            let lock = manifests::read(&root, &dir.join("package-lock.json"))?;
            let mut current = directory;
            loop {
                let relative = current
                    .strip_prefix(dir)
                    .ok()?
                    .to_string_lossy()
                    .replace('\\', "/");
                let key = format!(
                    "{}node_modules/{name}",
                    if relative.is_empty() {
                        String::new()
                    } else {
                        format!("{relative}/")
                    }
                );
                if let Some(value) = lock.get("packages").and_then(|v| v.get(&key)) {
                    if value.get("link").and_then(Value::as_bool) == Some(true) {
                        return None;
                    }
                    let version = value.get("version")?.as_str()?;
                    return super::super::workspace::node_version_matches(declared, version)
                        .then(|| version.to_owned());
                }
                if current == dir {
                    break;
                }
                current = current.parent()?;
            }
            if directory != dir {
                return None;
            }
            let version = lock
                .get("dependencies")?
                .get(&name)?
                .get("version")?
                .as_str()?;
            super::super::workspace::node_version_matches(declared, version)
                .then(|| version.to_owned())
        });
        describe(package, Some(declared), locked.as_deref(), dev)
    }
}
