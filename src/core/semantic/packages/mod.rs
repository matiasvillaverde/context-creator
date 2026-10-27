//! Offline metadata for direct external dependencies. Requirements are not installed versions.
use super::{analyzer::AnalysisResult, manifests};
use std::path::Path;
mod node;
mod python;
mod rust;
pub use node::NodePackageResolver;
pub use python::PythonPackageResolver;
pub use rust::CargoPackageResolver;

/// Resolve direct dependency metadata inside an explicit repository boundary.
/// `include_dev = false` omits dependencies found only in development sections.
pub trait PackageVersionResolver {
    fn resolve_version(
        &self,
        module: &str,
        source: &Path,
        root: &Path,
        include_dev: bool,
    ) -> Option<String>;
}

fn requirement(value: &str) -> Option<&str> {
    let value = value.trim();
    (value.len() <= 160
        && !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || " .^~+-*<>=!,|()".contains(c)))
    .then_some(value)
}
fn version(value: &str) -> Option<&str> {
    (value.len() <= 64
        && value.starts_with(|c: char| c.is_ascii_digit())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".+-_".contains(c)))
    .then_some(value)
}
fn describe(name: &str, declared: Option<&str>, locked: Option<&str>, dev: bool) -> Option<String> {
    if name.len() > 214
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@/_.-".contains(c))
    {
        return None;
    }
    let mut details = Vec::new();
    let mut label = name.to_owned();
    if let Some(locked) = locked.and_then(version) {
        label.push_str(&format!(" v{locked}"));
        details.push("locked".to_owned());
    }
    if let Some(declared) = declared.and_then(requirement) {
        details.push(format!("declared: {declared}"));
    }
    if dev {
        details.push("dev".into());
    }
    if !details.is_empty() {
        label.push_str(&format!(" ({})", details.join("; ")));
    }
    Some(label)
}

pub(crate) fn enrich(result: &mut AnalysisResult, source: &Path, root: &Path) {
    let Some(resolver) = super::get_resolver_for_file(source).ok().flatten() else {
        return;
    };
    let package_resolver: &dyn PackageVersionResolver = match source
        .extension()
        .and_then(|v| v.to_str())
    {
        Some("rs") => &CargoPackageResolver,
        Some("py" | "pyi" | "pyw") => &PythonPackageResolver,
        Some("js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts") => &NodePackageResolver,
        _ => return,
    };
    for reference in &mut result.type_references {
        if reference.definition_path.is_some() {
            continue;
        }
        let Some(module) = reference.module.as_deref() else {
            continue;
        };
        // A default/namespace import can be the qualifier in React.Component.
        let module = result
            .imports
            .iter()
            .find(|import| {
                import
                    .items
                    .iter()
                    .any(|item| item == module || item.strip_prefix("* as ") == Some(module))
            })
            .map(|import| import.module.as_str())
            .unwrap_or(module);
        if super::workspace::resolve_import(module, source, root).is_some() {
            continue;
        }
        if resolver
            .resolve_import(module, source, root)
            .ok()
            .is_some_and(|path| path.is_external)
        {
            reference.is_external = true;
            reference.external_package = package_resolver
                .resolve_version(module, source, root, true)
                .or_else(|| {
                    let name = match source.extension().and_then(|v| v.to_str()) {
                        Some("rs") => module.split("::").next()?,
                        Some("py" | "pyi" | "pyw") => module.split('.').next()?,
                        _ => module,
                    };
                    let name = if name.starts_with('@') {
                        name.split('/').take(2).collect::<Vec<_>>().join("/")
                    } else {
                        name.split('/').next()?.to_owned()
                    };
                    describe(&name, None, None, false)
                });
        }
    }
}
