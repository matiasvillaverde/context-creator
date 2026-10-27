use context_creator::core::semantic::{
    analyzer::{SemanticContext, TypeReference},
    get_analyzer_for_file,
    packages::{
        CargoPackageResolver, NodePackageResolver, PackageVersionResolver, PythonPackageResolver,
    },
};
use std::{fs, path::Path};
use tempfile::TempDir;
fn write(root: &Path, path: &str, value: &str) {
    let file = root.join(path);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, value).unwrap();
}
fn analyze(root: &Path, file: &str, source: &str) -> Vec<TypeReference> {
    write(root, file, source);
    let path = root.join(file);
    let context = SemanticContext::new(path.clone(), root.to_path_buf(), 5);
    get_analyzer_for_file(&path)
        .unwrap()
        .unwrap()
        .analyze_file(&path, source, &context)
        .unwrap()
        .type_references
}
fn metadata<'a>(refs: &'a [TypeReference], name: &str) -> &'a str {
    refs.iter()
        .find(|r| r.name == name && r.is_external)
        .and_then(|r| r.external_package.as_deref())
        .unwrap_or_else(|| panic!("missing metadata for {name}: {refs:?}"))
}
#[test]
fn cargo_versions_are_locked_only_when_unique_and_compatible() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    for requirement in ["1.0", "^1.0", "~1.0.190"] {
        write(
            root,
            "Cargo.toml",
            &format!(
                "[package]\nname='app'\nversion='0.1.0'\n[dependencies]\nserde='{requirement}'\n"
            ),
        );
        write(
            root,
            "Cargo.lock",
            "version=3\n[[package]]\nname='serde'\nversion='1.0.197'\n",
        );
        let refs = analyze(
            root,
            "src/lib.rs",
            "use serde::Serialize;\npub fn output<T: Serialize>(value:T) {}\n",
        );
        assert!(metadata(&refs, "Serialize").contains("serde v1.0.197 (locked; declared:"));
    }
    write(root,"Cargo.lock","[[package]]\nname='serde'\nversion='1.0.197'\n[[package]]\nname='serde'\nversion='1.0.198'\n");
    let value = CargoPackageResolver
        .resolve_version("serde", &root.join("src/lib.rs"), root, true)
        .unwrap();
    assert!(!value.contains("locked"));
    assert!(value.contains("declared:"));
}
#[test]
fn python_requirements_pyproject_and_static_setup_are_declared() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let source = "from django.db.models import Model\nclass Account(Model):\n    pass\n";
    write(root, "requirements.txt", "# versions\ndjango==3.2.0\n");
    assert_eq!(
        metadata(&analyze(root, "service.py", source), "Model"),
        "django (declared: ==3.2.0)"
    );
    fs::remove_file(root.join("requirements.txt")).unwrap();
    write(root,"pyproject.toml","[project]\nname='app'\nversion='0.1'\ndependencies=['django>=3.2,<4; python_version >= \"3.8\"']\n");
    assert_eq!(
        metadata(&analyze(root, "service.py", source), "Model"),
        "django (declared: >=3.2,<4)"
    );
    fs::remove_file(root.join("pyproject.toml")).unwrap();
    write(root,"setup.py","from setuptools import setup\nsetup(name='app',install_requires=['django~=3.2'])\nraise RuntimeError('must never execute')\n");
    assert_eq!(
        metadata(&analyze(root, "service.py", source), "Model"),
        "django (declared: ~=3.2)"
    );
}
#[test]
fn javascript_and_typescript_qualified_types_use_direct_package_lock() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "package-lock.json",
        r#"{"lockfileVersion":3,"packages":{"node_modules/react":{"version":"16.14.0"}}}"#,
    );
    for requirement in ["^16.0.0", "~16.14.0", ">=16.0.0"] {
        write(
            root,
            "package.json",
            &format!(r#"{{"dependencies":{{"react":"{requirement}"}}}}"#),
        );
        for (path, source) in [
            (
                "service.jsx",
                "import React from 'react'; export class View extends React.Component {}",
            ),
            (
                "service.ts",
                "import * as React from 'react'; const view: React.Component = null;",
            ),
        ] {
            let refs = analyze(root, path, source);
            assert!(metadata(&refs, "Component").starts_with("react v16.14.0 (locked; declared:"));
        }
    }
    write(
        root,
        "package.json",
        r#"{"dependencies":{"react":"^18.0.0"}}"#,
    );
    assert_eq!(
        NodePackageResolver
            .resolve_version("react", &root.join("service.jsx"), root, true)
            .unwrap(),
        "react (declared: ^18.0.0)"
    );
}
#[test]
fn workspace_versions_use_member_declarations_and_inherited_cargo_aliases() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root,"Cargo.toml","[workspace]\nmembers=['app']\n[workspace.dependencies]\nserialization={package='serde',version='1.0'}\n");
    write(
        root,
        "app/Cargo.toml",
        "[package]\nname='app'\nversion='0.1.0'\n[dependencies]\nserialization={workspace=true}\n",
    );
    write(
        root,
        "Cargo.lock",
        "[[package]]\nname='serde'\nversion='1.0.197'\n",
    );
    let refs = analyze(
        root,
        "app/src/lib.rs",
        "use serialization::Serialize;\npub fn run<T: Serialize>(value:T) {}\n",
    );
    assert!(metadata(&refs, "Serialize").starts_with("serde v1.0.197"));
    write(
        root,
        "package.json",
        r#"{"workspaces":["packages/*"],"dependencies":{"unrelated":"9.0"}}"#,
    );
    write(
        root,
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"react":"^18.0.0"}}"#,
    );
    write(
        root,
        "package-lock.json",
        r#"{"packages":{"node_modules/react":{"version":"18.2.0"}}}"#,
    );
    let refs = analyze(
        root,
        "packages/app/view.jsx",
        "import React from 'react'; class View extends React.Component {}",
    );
    assert!(metadata(&refs, "Component").starts_with("react v18.2.0"));
    assert!(NodePackageResolver
        .resolve_version("unrelated", &root.join("packages/app/view.jsx"), root, true)
        .is_none());
}
#[test]
fn missing_invalid_and_oversized_manifests_keep_external_name_without_version() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    for content in [None, Some("not TOML"), Some("x")] {
        if let Some(value) = content {
            write(root, "Cargo.toml", value);
        }
        let refs = analyze(
            root,
            "lib.rs",
            "use serde::Serialize;\nfn run<T: Serialize>(value:T) {}\n",
        );
        assert_eq!(metadata(&refs, "Serialize"), "serde");
    }
    write(root, "Cargo.toml", &"a".repeat(1024 * 1024 + 1));
    let refs = analyze(
        root,
        "lib.rs",
        "use serde::Serialize;\nfn run<T: Serialize>(value:T) {}\n",
    );
    assert_eq!(metadata(&refs, "Serialize"), "serde");
}
#[test]
fn same_size_manifest_and_lock_edits_refresh_cached_versions() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "package.json",
        r#"{"dependencies":{"react":"^18.0.0"}}"#,
    );
    for version in ["18.2.0", "18.3.0"] {
        write(
            root,
            "package-lock.json",
            &format!(r#"{{"packages":{{"node_modules/react":{{"version":"{version}"}}}}}}"#),
        );
        let refs = analyze(
            root,
            "view.jsx",
            "import React from 'react'; class View extends React.Component {}",
        );
        assert!(metadata(&refs, "Component").contains(version));
    }
    write(
        root,
        "package.json",
        r#"{"dependencies":{"react":"^19.0.0"}}"#,
    );
    let refs = analyze(
        root,
        "view.jsx",
        "import React from 'react'; class View extends React.Component {}",
    );
    assert_eq!(metadata(&refs, "Component"), "react (declared: ^19.0.0)");
}
#[test]
fn dev_dependencies_are_marked_and_can_be_excluded_by_resolvers() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[package]\nname='app'\nversion='0.1.0'\n[dev-dependencies]\nserde='1.0'\n",
    );
    write(root, "lib.rs", "");
    assert!(CargoPackageResolver
        .resolve_version("serde", &root.join("lib.rs"), root, true)
        .unwrap()
        .contains("dev"));
    assert!(CargoPackageResolver
        .resolve_version("serde", &root.join("lib.rs"), root, false)
        .is_none());
    write(
        root,
        "package.json",
        r#"{"devDependencies":{"react":"^18.0.0"}}"#,
    );
    write(root, "view.js", "");
    assert!(NodePackageResolver
        .resolve_version("react", &root.join("view.js"), root, true)
        .unwrap()
        .contains("dev"));
    assert!(NodePackageResolver
        .resolve_version("react", &root.join("view.js"), root, false)
        .is_none());
    write(
        root,
        "pyproject.toml",
        "[tool.poetry.group.dev.dependencies]\ndjango='^3.2'\n",
    );
    write(root, "app.py", "");
    assert!(PythonPackageResolver
        .resolve_version("django", &root.join("app.py"), root, true)
        .unwrap()
        .contains("dev"));
    assert!(PythonPackageResolver
        .resolve_version("django", &root.join("app.py"), root, false)
        .is_none());
}
#[test]
fn transitive_dependencies_and_urls_never_gain_versions_or_leak_credentials() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "package.json",
        &serde_json::json!({"dependencies": {"react": format!("https://{}:{}@example.test/react.tgz", "fixture-user", "fixture-only")}}).to_string(),
    );
    write(
        root,
        "package-lock.json",
        r#"{"packages":{"node_modules/transitive":{"version":"1.2.3"}}}"#,
    );
    write(root, "view.js", "");
    assert!(NodePackageResolver
        .resolve_version("transitive", &root.join("view.js"), root, true)
        .is_none());
    assert_eq!(
        NodePackageResolver
            .resolve_version("react", &root.join("view.js"), root, true)
            .unwrap(),
        "react"
    );
    let outside = TempDir::new().unwrap();
    write(outside.path(), "app.rs", "");
    assert!(CargoPackageResolver
        .resolve_version("serde", &outside.path().join("app.rs"), root, true)
        .is_none());
}
#[test]
fn cli_formats_and_agent_gaps_expose_dependency_metadata_within_budget() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[package]\nname='app'\nversion='0.1.0'\n[dependencies]\nserde='1.0'\n",
    );
    write(
        root,
        "Cargo.lock",
        "[[package]]\nname='serde'\nversion='1.0.197'\n",
    );
    write(
        root,
        "src/lib.rs",
        "use serde::Serialize;\npub fn output<T: Serialize>(value:T) {}\n",
    );
    for style in ["markdown", "plain", "xml"] {
        let output = assert_cmd::Command::cargo_bin("context-creator")
            .unwrap()
            .arg(root.join("src"))
            .args(["--include-types", "--style", style])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        assert!(String::from_utf8(output)
            .unwrap()
            .contains("serde v1.0.197"));
    }
    let mut request = context_creator::agent::ContextRequest::new(root, "output");
    request.seeds.push("src/lib.rs".into());
    let response = context_creator::agent::AgentService::new()
        .build_context(request)
        .unwrap();
    assert!(response
        .gaps
        .iter()
        .any(|g| g.kind == "external_type" && g.detail.contains("serde v1.0.197")));
    assert!(response.token_count <= 8000);
}
