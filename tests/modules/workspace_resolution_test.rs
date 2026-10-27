use context_creator::core::semantic::{
    get_analyzer_for_file, get_resolver_for_file, SemanticContext,
};
use std::{fs, path::Path};
use tempfile::TempDir;

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn check_type(root: &Path, source: &str, module: &str, name: &str, expected: &str) {
    let root = root.canonicalize().unwrap();
    let path = root.join(source);
    let resolved = get_resolver_for_file(&path)
        .unwrap()
        .unwrap()
        .resolve_import(module, &path, &root)
        .unwrap();
    assert!(!resolved.is_external, "{module} must be local");
    let analysis = get_analyzer_for_file(&path)
        .unwrap()
        .unwrap()
        .analyze_file(
            &path,
            &fs::read_to_string(&path).unwrap(),
            &SemanticContext::new(path.clone(), root.clone(), 3),
        )
        .unwrap();
    let expected = root.join(expected).canonicalize().unwrap();
    assert!(
        analysis.type_references.iter().any(|reference| {
            reference.name == name
                && !reference.is_external
                && reference.definition_path.as_ref() == Some(&expected)
        }),
        "{name} must resolve to {expected:?}: {:?}",
        analysis.type_references
    );
}

fn rust_workspace(root: &Path) {
    write(root, "Cargo.toml", "[workspace]\nmembers=['app','models']\nresolver='2'\n[workspace.dependencies]\nmodels={package='domain-models',path='models'}\n");
    write(
        root,
        "models/Cargo.toml",
        "[package]\nname='domain-models'\nversion='0.1.0'\n[lib]\npath='source/domain.rs'\n",
    );
    write(
        root,
        "models/source/domain.rs",
        "pub struct Account { pub id: u64 }\n",
    );
    write(
        root,
        "app/Cargo.toml",
        "[package]\nname='application'\nversion='0.1.0'\n[dependencies]\nmodels.workspace=true\n",
    );
    write(
        root,
        "app/src/lib.rs",
        "use models::Account;\npub fn first(value: Account) {}\npub fn second(value: Account) {}\n",
    );
}

#[test]
fn rust_workspace_inheritance_alias_and_custom_library_path() {
    let dir = TempDir::new().unwrap();
    rust_workspace(dir.path());
    check_type(
        dir.path(),
        "app/src/lib.rs",
        "models",
        "Account",
        "models/source/domain.rs",
    );
}

#[test]
fn python_poetry_path_dependency_supports_src_layout() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "pyproject.toml", "[tool.poetry]\nname='app'\n[tool.poetry.group.dev.dependencies]\nmodels={path='packages/models',develop=true}\n");
    write(
        root,
        "packages/models/pyproject.toml",
        "[project]\nname='models'\nversion='0.1.0'\n",
    );
    write(root, "packages/models/src/models/__init__.py", "");
    write(
        root,
        "packages/models/src/models/user.py",
        "class User:\n    id: int\n",
    );
    write(
        root,
        "app/service.py",
        "from models.user import User\ndef process(value: User) -> User:\n    return value\n",
    );
    check_type(
        root,
        "app/service.py",
        "models.user",
        "User",
        "packages/models/src/models/user.py",
    );
}

#[test]
fn npm_and_yarn_workspace_packages_resolve_types() {
    for workspaces in ["[\"packages/*\"]", "{\"packages\":[\"packages/*\"]}"] {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write(
            root,
            "package.json",
            &format!("{{\"private\":true,\"workspaces\":{workspaces}}}"),
        );
        write(
            root,
            "packages/types/package.json",
            "{\"name\":\"@app/types\",\"version\":\"1.0.0\",\"types\":\"src/index.ts\"}",
        );
        write(
            root,
            "packages/types/src/index.ts",
            "export interface User { id: string; }\n",
        );
        write(
            root,
            "packages/app/package.json",
            "{\"name\":\"@app/client\",\"dependencies\":{\"@app/types\":\"1.0.0\"}}",
        );
        write(root, "packages/app/src/service.ts", "import { User } from '@app/types';\nexport function process(user: User): User { return user; }\n");
        check_type(
            root,
            "packages/app/src/service.ts",
            "@app/types",
            "User",
            "packages/types/src/index.ts",
        );
    }
}

#[test]
fn pnpm_workspace_and_subpath_exports_resolve_types() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "pnpm-workspace.yaml",
        "packages:\n  - 'packages/*'\n  - '!packages/excluded'\n",
    );
    write(
        root,
        "packages/types/package.json",
        "{\"name\":\"@app/types\",\"exports\":{\"./user\":{\"types\":\"./src/user.ts\"}}}",
    );
    write(
        root,
        "packages/types/src/user.ts",
        "export interface User { id: string; }\n",
    );
    write(
        root,
        "packages/client/package.json",
        "{\"name\":\"client\",\"dependencies\":{\"@app/types\":\"workspace:*\"}}",
    );
    write(root, "packages/client/service.ts", "import { User } from '@app/types/user';\nexport function process(user: User): User { return user; }\n");
    check_type(
        root,
        "packages/client/service.ts",
        "@app/types/user",
        "User",
        "packages/types/src/user.ts",
    );
}

#[test]
fn cli_includes_sibling_types_without_false_cycles_and_respects_ignores() {
    use assert_cmd::Command;
    let dir = TempDir::new().unwrap();
    rust_workspace(dir.path());
    let output = Command::cargo_bin("context-creator")
        .unwrap()
        .arg(dir.path().join("app/src"))
        .args(["--include-types", "-v", "--max-tokens", "8000"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("pub struct Account"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Circular type reference"));
    write(dir.path(), ".contextignore", "models/\n");
    let output = Command::cargo_bin("context-creator")
        .unwrap()
        .arg(dir.path().join("app/src"))
        .args(["--include-types", "--max-tokens", "8000"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("pub struct Account"));
}

#[test]
fn rust_reexports_and_imported_type_aliases_reach_the_definition() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    rust_workspace(root);
    write(
        root,
        "models/source/domain.rs",
        "mod account;\npub use crate::account::Account;\n",
    );
    write(
        root,
        "models/source/account.rs",
        "pub struct Account { pub id: u64 }\n",
    );
    for (import, module) in [
        ("use models::Account as Customer;", "models::Account"),
        ("use models::{Account as Customer};", "models"),
    ] {
        write(
            root,
            "app/src/lib.rs",
            &format!("{import}\npub fn process(value: Customer) {{}}\n"),
        );
        check_type(
            root,
            "app/src/lib.rs",
            module,
            "Customer",
            "models/source/account.rs",
        );
    }
}

#[test]
fn python_import_alias_keeps_the_original_definition() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "pyproject.toml",
        "[tool.poetry.dependencies]\nmodels={path='packages/models'}\n",
    );
    write(root, "packages/models/__init__.py", "");
    write(
        root,
        "packages/models/user.py",
        "class User:\n    id: int\n",
    );
    write(root, "app.py", "from models.user import User as Customer\ndef process(value: Customer) -> Customer:\n    return value\n");
    check_type(
        root,
        "app.py",
        "models.user",
        "Customer",
        "packages/models/user.py",
    );
}

#[test]
fn typescript_reexport_and_import_aliases_reach_the_definition() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "package.json", "{\"workspaces\":[\"packages/*\"]}");
    write(
        root,
        "packages/types/package.json",
        "{\"name\":\"@app/types\",\"types\":\"src/index.ts\"}",
    );
    write(
        root,
        "packages/types/src/index.ts",
        "export { User as Person } from './user';\n",
    );
    write(
        root,
        "packages/types/src/user.ts",
        "export interface User { id: string; }\n",
    );
    write(
        root,
        "packages/app/package.json",
        "{\"dependencies\":{\"@app/types\":\"workspace:*\"}}",
    );
    write(root, "packages/app/service.ts", "import { Person as Customer } from '@app/types';\nexport function process(value: Customer): Customer { return value; }\n");
    check_type(
        root,
        "packages/app/service.ts",
        "@app/types",
        "Customer",
        "packages/types/src/user.ts",
    );
}

#[test]
fn manifest_edit_changes_binding_without_changing_source() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    for target in ["left", "rght"] {
        write(
            root,
            &format!("{target}/Cargo.toml"),
            "[package]\nname='models'\nversion='0.1.0'\n",
        );
        write(
            root,
            &format!("{target}/src/lib.rs"),
            "pub struct Account { pub id: u64 }\n",
        );
    }
    write(
        root,
        "app/src/lib.rs",
        "use models::Account;\npub fn process(value: Account) {}\n",
    );
    for target in ["left", "rght"] {
        write(root, "app/Cargo.toml", &format!("[package]\nname='app'\nversion='0.1.0'\n[dependencies]\nmodels={{path='../{target}'}}\n"));
        check_type(
            root,
            "app/src/lib.rs",
            "models",
            "Account",
            &format!("{target}/src/lib.rs"),
        );
    }
}

#[test]
fn ambiguous_workspace_names_do_not_guess_a_package() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    write(&root, "package.json", "{\"workspaces\":[\"packages/*\"]}");
    for target in ["one", "two"] {
        write(
            &root,
            &format!("packages/{target}/package.json"),
            "{\"name\":\"@app/types\",\"types\":\"src/index.ts\"}",
        );
        write(
            &root,
            &format!("packages/{target}/src/index.ts"),
            "export interface User { id: string; }\n",
        );
    }
    write(
        &root,
        "packages/app/package.json",
        "{\"dependencies\":{\"@app/types\":\"workspace:*\"}}",
    );
    write(
        &root,
        "packages/app/service.ts",
        "import { User } from '@app/types';\n",
    );
    let path = root.join("packages/app/service.ts");
    assert!(
        get_resolver_for_file(&path)
            .unwrap()
            .unwrap()
            .resolve_import("@app/types", &path, &root)
            .unwrap()
            .is_external
    );
}

#[test]
fn local_dependency_cannot_escape_explicit_root() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let outside = TempDir::new().unwrap();
    write(
        outside.path(),
        "Cargo.toml",
        "[package]\nname='models'\nversion='0.1.0'\n",
    );
    write(
        outside.path(),
        "src/lib.rs",
        "pub struct Account { pub id: u64 }\n",
    );
    write(
        &root,
        "Cargo.toml",
        &format!(
            "[package]\nname='app'\nversion='0.1.0'\n[dependencies]\nmodels={{path={:?}}}\n",
            outside.path().to_string_lossy()
        ),
    );
    write(
        &root,
        "src/lib.rs",
        "use models::Account;\npub fn process(value: Account) {}\n",
    );
    let path = root.join("src/lib.rs");
    let resolved = get_resolver_for_file(&path)
        .unwrap()
        .unwrap()
        .resolve_import("models", &path, &root)
        .unwrap();
    assert!(resolved.is_external);
    let result = get_analyzer_for_file(&path)
        .unwrap()
        .unwrap()
        .analyze_file(
            &path,
            &fs::read_to_string(&path).unwrap(),
            &SemanticContext::new(path.clone(), root.clone(), 3),
        )
        .unwrap();
    assert!(result.type_references.iter().all(|reference| reference
        .definition_path
        .as_ref()
        .is_none_or(|path| path.starts_with(&root))));
}

#[cfg(unix)]
#[test]
fn workspace_does_not_follow_member_symlinks() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    rust_workspace(&root);
    fs::rename(root.join("models"), root.join("actual-models")).unwrap();
    std::os::unix::fs::symlink(root.join("actual-models"), root.join("models")).unwrap();
    let path = root.join("app/src/lib.rs");
    assert!(
        get_resolver_for_file(&path)
            .unwrap()
            .unwrap()
            .resolve_import("models", &path, &root)
            .unwrap()
            .is_external
    );
}

#[test]
fn python_static_setup_dependency_is_read_without_executing_code() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "setup.py", "from setuptools import setup\nraise RuntimeError('must never execute')\nsetup(name='app',install_requires=['models @ file:packages/models'])\n");
    write(root, "packages/models/__init__.py", "");
    write(
        root,
        "packages/models/user.py",
        "class User:\n    id: int\n",
    );
    write(
        root,
        "service.py",
        "from models.user import User\ndef process(value: User) -> User:\n    return value\n",
    );
    check_type(
        root,
        "service.py",
        "models.user",
        "User",
        "packages/models/user.py",
    );
}

#[test]
fn python_uv_workspace_resolves_declared_member() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "pyproject.toml",
        "[tool.uv.workspace]\nmembers=['packages/*']\n[tool.uv.sources]\nmodels={workspace=true}\n",
    );
    write(
        root,
        "packages/models/pyproject.toml",
        "[project]\nname='models'\nversion='0.1.0'\n",
    );
    write(
        root,
        "packages/models/src/models/user.py",
        "class User:\n    id: int\n",
    );
    write(
        root,
        "packages/app/pyproject.toml",
        "[project]\nname='app'\nversion='0.1.0'\ndependencies=['models']\n",
    );
    write(
        root,
        "packages/app/service.py",
        "from models.user import User\ndef process(value: User) -> User:\n    return value\n",
    );
    check_type(
        root,
        "packages/app/service.py",
        "models.user",
        "User",
        "packages/models/src/models/user.py",
    );
}

#[test]
fn recursive_workspace_patterns_find_nested_members() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "package.json", "{\"workspaces\":[\"packages/**\"]}");
    write(
        root,
        "packages/domain/types/package.json",
        "{\"name\":\"@app/types\",\"types\":\"index.ts\"}",
    );
    write(
        root,
        "packages/domain/types/index.ts",
        "export interface User { id: string; }\n",
    );
    write(
        root,
        "packages/client/package.json",
        "{\"dependencies\":{\"@app/types\":\"workspace:*\"}}",
    );
    write(root, "packages/client/service.ts", "import { User } from '@app/types';\nexport function process(value: User): User { return value; }\n");
    check_type(
        root,
        "packages/client/service.ts",
        "@app/types",
        "User",
        "packages/domain/types/index.ts",
    );
}

#[test]
fn malformed_manifests_do_not_discard_source_analysis() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    write(&root, "package.json", "not json");
    write(&root, "service.ts", "import { User } from '@app/types';\nexport function process(value: User): User { return value; }\n");
    let path = root.join("service.ts");
    let result = get_analyzer_for_file(&path)
        .unwrap()
        .unwrap()
        .analyze_file(
            &path,
            &fs::read_to_string(&path).unwrap(),
            &SemanticContext::new(path.clone(), root, 3),
        )
        .unwrap();
    assert!(result
        .exported_functions
        .iter()
        .any(|function| function.name == "process"));
}

#[test]
fn npm_workspace_version_requirements_do_not_bind_incompatible_packages() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    write(&root, "package.json", "{\"workspaces\":[\"packages/*\"]}");
    write(
        &root,
        "packages/types/package.json",
        "{\"name\":\"@app/types\",\"version\":\"1.2.3\",\"types\":\"index.ts\"}",
    );
    write(
        &root,
        "packages/types/index.ts",
        "export interface User { id: string; }\n",
    );
    write(
        &root,
        "packages/app/service.ts",
        "import { User } from '@app/types';\n",
    );
    for (requirement, local) in [
        ("^1.0.0", true),
        ("~1.2.0", true),
        ("1.2.3", true),
        (">=1.0.0 <2.0.0", true),
        ("1.0.0", false),
        ("^2.0.0", false),
        ("1.1", false),
    ] {
        write(
            &root,
            "packages/app/package.json",
            &format!("{{\"dependencies\":{{\"@app/types\":{requirement:?}}}}}"),
        );
        let path = root.join("packages/app/service.ts");
        let result = get_resolver_for_file(&path)
            .unwrap()
            .unwrap()
            .resolve_import("@app/types", &path, &root)
            .unwrap();
        assert_eq!(!result.is_external, local, "{requirement}");
    }
}

#[test]
fn dependency_source_edits_do_not_reuse_stale_declarations() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    rust_workspace(&root);
    check_type(
        &root,
        "app/src/lib.rs",
        "models",
        "Account",
        "models/source/domain.rs",
    );
    write(
        &root,
        "models/source/domain.rs",
        "pub struct Removed { pub id: u64 }\n",
    );
    let path = root.join("app/src/lib.rs");
    let result = get_analyzer_for_file(&path)
        .unwrap()
        .unwrap()
        .analyze_file(
            &path,
            &fs::read_to_string(&path).unwrap(),
            &SemanticContext::new(path.clone(), root, 3),
        )
        .unwrap();
    assert!(result
        .type_references
        .iter()
        .filter(|reference| reference.name == "Account")
        .all(|reference| reference.definition_path.is_none()));
}

#[test]
fn agent_session_rebuilds_workspace_edges_after_manifest_edit() {
    use context_creator::agent::{AgentService, ContextRequest};
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    for target in ["left", "rght"] {
        write(
            root,
            &format!("{target}/Cargo.toml"),
            "[package]\nname='models'\nversion='0.1.0'\n",
        );
        write(
            root,
            &format!("{target}/src/lib.rs"),
            "pub struct Account { pub id: u64 }\n",
        );
    }
    write(
        root,
        "app/src/lib.rs",
        "use models::Account;\npub fn process(value: Account) {}\n",
    );
    let service = AgentService::new();
    let mut request = ContextRequest::new(root, "");
    request.seeds.push("app/src/lib.rs".into());
    request.session_id = Some("workspace".into());
    for target in ["left", "rght"] {
        write(root, "app/Cargo.toml", &format!("[package]\nname='app'\nversion='0.1.0'\n[dependencies]\nmodels={{path='../{target}'}}\n"));
        let response = service.build_context(request.clone()).unwrap();
        assert!(response
            .relations
            .iter()
            .any(|relation| relation.from.path == "app/src/lib.rs"
                && relation.to.path == format!("{target}/src/lib.rs")));
        request.expected_revision = Some(response.revision);
    }
}

#[test]
fn pnpm_quoted_inline_member_list_resolves_local_types() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "pnpm-workspace.yaml", "packages: ['packages/*']\n");
    write(
        root,
        "packages/models/package.json",
        "{\"name\":\"models\",\"types\":\"index.ts\"}",
    );
    write(
        root,
        "packages/models/index.ts",
        "export interface User { id: string; }\n",
    );
    write(
        root,
        "packages/app/package.json",
        "{\"dependencies\":{\"models\":\"workspace:*\"}}",
    );
    write(root, "packages/app/main.ts", "import { User } from 'models';\nexport function process(value: User): User { return value; }\n");
    check_type(
        root,
        "packages/app/main.ts",
        "models",
        "User",
        "packages/models/index.ts",
    );
}
