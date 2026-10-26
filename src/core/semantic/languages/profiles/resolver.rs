use super::Profile;
use crate::{
    core::semantic::{
        analyzer::SemanticResult,
        path_validator::validate_import_path,
        resolver::{ModuleResolver, ResolvedPath},
    },
    utils::error::ContextCreatorError,
};
use std::path::{Path, PathBuf};

pub struct ProfileResolver(pub Profile);
impl ModuleResolver for ProfileResolver {
    fn is_external_module(&self, module: &str) -> bool {
        module.contains("://") || module.starts_with("dart:")
    }
    fn get_file_extensions(&self) -> Vec<&'static str> {
        self.0.extensions()
    }
    fn resolve_import(
        &self,
        module: &str,
        from: &Path,
        root: &Path,
    ) -> SemanticResult<ResolvedPath> {
        if module.is_empty()
            || module.len() > 4096
            || module.contains(['\0', '\n', '\r', '`', '<', '>'])
        {
            return Err(ContextCreatorError::SecurityError(
                "Invalid module path".into(),
            ));
        }
        let profile = self.0;
        let module = if matches!(profile, Profile::Html | Profile::Css) {
            module.split(['?', '#']).next().unwrap_or(module)
        } else {
            module
        };
        if module.contains("://")
            || ["git::", "dart:", "data:"]
                .iter()
                .any(|prefix| module.starts_with(prefix))
        {
            return Ok(external(module));
        }
        let mut candidates = Vec::new();
        let mut confidence = 0.75;
        let parent = from.parent().unwrap_or(root);
        match profile {
            Profile::R
                if !module.contains('/')
                    && !Path::new(module)
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("r")) =>
            {
                return Ok(external(module))
            }
            Profile::Html | Profile::Css if module.starts_with("//") => {
                return Ok(external(module))
            }
            Profile::Html | Profile::Css if module.starts_with('/') => {
                for base in [root.to_path_buf(), root.join("public"), root.join("static")] {
                    candidates.push(base.join(module.trim_start_matches('/')));
                }
            }
            Profile::Dart if module.starts_with("package:") => {
                let (package, path) = module
                    .trim_start_matches("package:")
                    .split_once('/')
                    .ok_or_else(|| {
                        ContextCreatorError::InvalidPath("Invalid Dart package import".into())
                    })?;
                let local = dart_manifest(root).is_some_and(|source| {
                    source.lines().any(|line| {
                        line.strip_prefix("name:")
                            .is_some_and(|name| name.trim().trim_matches(['\'', '"']) == package)
                    })
                });
                if !local {
                    return Ok(external(module));
                }
                candidates.push(root.join("lib").join(path));
                confidence = 1.0;
            }
            Profile::Scala
            | Profile::Haskell
            | Profile::Clojure
            | Profile::Elixir
            | Profile::Perl
            | Profile::Lua
                if !module.contains('/')
                    && !Path::new(module).is_absolute()
                    && !Path::new(module).extension().is_some_and(|extension| {
                        profile
                            .extensions()
                            .iter()
                            .any(|candidate| extension == *candidate)
                    }) =>
            {
                let module = match profile {
                    Profile::Perl => module.replace("::", "/"),
                    Profile::Elixir => module
                        .split('.')
                        .map(snake_case)
                        .collect::<Vec<_>>()
                        .join("/"),
                    Profile::Clojure => module.replace('.', "/").replace('-', "_"),
                    _ => module.replace('.', "/"),
                };
                for base in [
                    parent.to_path_buf(),
                    root.to_path_buf(),
                    root.join("src"),
                    root.join("lib"),
                    root.join("lua"),
                    root.join("src/main/scala"),
                    root.join("app"),
                ] {
                    let extensions = if profile == Profile::Perl
                        && !module.ends_with(".pm")
                        && !module.ends_with(".pl")
                    {
                        vec!["pm"]
                    } else {
                        profile.extensions()
                    };
                    for extension in extensions {
                        candidates.push(base.join(&module).with_extension(extension));
                        if profile == Profile::Lua {
                            candidates.push(base.join(&module).join("init.lua"));
                        }
                    }
                    if profile == Profile::Scala {
                        candidates.push(base.join(&module));
                    }
                }
            }
            Profile::Julia if !module.contains('/') && !module.ends_with(".jl") => {
                let module = module.trim_start_matches('.');
                candidates.push(parent.join(module).with_extension("jl"));
                candidates.push(root.join("src").join(module).with_extension("jl"));
            }
            Profile::Sql => return Ok(external(module)),
            _ => {
                if module.contains(':') || Path::new(module).is_absolute() {
                    return Err(ContextCreatorError::SecurityError(
                        "File imports must be relative".into(),
                    ));
                }
                candidates.push(parent.join(module));
                candidates.push(root.join(module));
                if matches!(
                    profile,
                    Profile::Dart | Profile::Julia | Profile::Css | Profile::Html | Profile::Hcl
                ) {
                    confidence = 1.0;
                }
            }
        }
        for candidate in candidates {
            if !candidate.exists() {
                continue;
            }
            let path = validate_import_path(root, &candidate)?;
            for part in candidate.ancestors().take_while(|part| *part != root) {
                if std::fs::symlink_metadata(part)
                    .map_err(ContextCreatorError::IoError)?
                    .file_type()
                    .is_symlink()
                {
                    return Err(ContextCreatorError::SecurityError(
                        "Module imports cannot follow symlinks".into(),
                    ));
                }
            }
            if path.is_file() || matches!(profile, Profile::Hcl | Profile::Scala) && path.is_dir() {
                return Ok(ResolvedPath {
                    path,
                    is_external: false,
                    confidence,
                });
            }
        }
        if module.starts_with('.')
            || module.contains('/')
            || Path::new(module).extension().is_some_and(|extension| {
                profile
                    .extensions()
                    .iter()
                    .any(|candidate| extension == *candidate)
            })
        {
            Err(ContextCreatorError::InvalidPath(
                "No visible local dependency file".into(),
            ))
        } else {
            Ok(external(module))
        }
    }
}
fn dart_manifest(root: &Path) -> Option<String> {
    use std::io::Read;
    let path = root.join("pubspec.yaml");
    if std::fs::symlink_metadata(&path)
        .ok()?
        .file_type()
        .is_symlink()
    {
        return None;
    }
    let path = validate_import_path(root, &path).ok()?;
    let mut source = String::new();
    std::fs::File::open(path)
        .ok()?
        .take(1_048_577)
        .read_to_string(&mut source)
        .ok()?;
    (source.len() <= 1_048_576).then_some(source)
}

fn external(module: &str) -> ResolvedPath {
    ResolvedPath {
        path: PathBuf::from(module),
        is_external: true,
        confidence: 0.0,
    }
}
fn snake_case(name: &str) -> String {
    let mut out = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && index > 0 {
            out.push('_');
        }
        out.extend(ch.to_lowercase());
    }
    out
}
