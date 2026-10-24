use context_creator::core::semantic::{
    get_analyzer_for_file, get_resolver_for_file, SemanticContext,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

struct Fixture {
    project: TempDir,
    entry: PathBuf,
    dependency: PathBuf,
}

fn fixture(language: &str) -> Fixture {
    let project = TempDir::new().unwrap();
    fs::create_dir(project.path().join(".git")).unwrap();
    let (entry, source, dependency, imported) = match language {
        "c" => ("Main.c", "#include \"user.h\"\nint run(void) { User user; return greet(&user); }\n", "user.h", "typedef struct User { int id; } User;\nint greet(User *user);\n"),
        "cpp" => ("Main.cpp", "#include \"user.hpp\"\nint run() { User user; return greet(user); }\n", "user.hpp", "class User { public: int id; };\nint greet(User user);\n"),
        "cs" => ("Main.cs", "using App.Models;\npublic class Main { public User run() { return User.create(); } }\n", "Models/User.cs", "namespace App.Models { public class User { public static User create() { return new User(); } } }\n"),
        "kt" => ("src/main/kotlin/app/Main.kt", "package app\nimport app.models.User\nfun run(): User { return User(1) }\n", "src/main/kotlin/app/models/User.kt", "package app.models\ndata class User(val id: Int)\n"),
        "rb" => ("app/Main.rb", "require_relative '../lib/user'\ndef run\n  User.new.name\nend\n", "lib/user.rb", "class User\n  def name\n    'Ada'\n  end\nend\n"),
        "php" => ("Main.php", "<?php\nuse App\\Models\\User;\nfunction run(User $user): User { greet(); return $user; }\n", "src/Models/User.php", "<?php\nnamespace App\\Models;\nclass User { public function greet(): string { return 'Ada'; } }\n"),
        _ => panic!("unknown fixture"),
    };
    if language == "php" {
        fs::write(
            project.path().join("composer.json"),
            r#"{"autoload":{"psr-4":{"App\\":"src/"}}}"#,
        )
        .unwrap();
    }
    for (path, content) in [(entry, source), (dependency, imported)] {
        let path = project.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    Fixture {
        entry: project.path().join(entry),
        dependency: project.path().join(dependency),
        project,
    }
}

#[test]
fn additional_languages_extract_real_ast_relationships() {
    for language in ["c", "cpp", "cs", "kt", "rb", "php"] {
        let fixture = fixture(language);
        let source = fs::read_to_string(&fixture.entry).unwrap();
        let analyzer = get_analyzer_for_file(&fixture.entry).unwrap().unwrap();
        let context = SemanticContext::new(
            fixture.entry.clone(),
            fixture.project.path().to_path_buf(),
            3,
        );
        let result = analyzer
            .analyze_file(&fixture.entry, &source, &context)
            .unwrap();
        assert!(result.errors.is_empty(), "{language}: {:?}", result.errors);
        assert!(!result.imports.is_empty(), "{language}: no imports");
        assert!(
            result
                .exported_functions
                .iter()
                .any(|function| function.name == "run"),
            "{language}: {:?}",
            result.exported_functions
        );
        assert!(!result.function_calls.is_empty(), "{language}: no calls");
        assert!(
            result
                .type_references
                .iter()
                .any(|reference| reference.name == "User"
                    && reference.definition_path.as_deref() == Some(fixture.dependency.as_path())),
            "{language}: {:?}",
            result.type_references
        );
    }
}

#[test]
fn additional_languages_expand_local_imports_and_types_from_the_cli() {
    for language in ["c", "cpp", "cs", "kt", "rb", "php"] {
        let fixture = fixture(language);
        for feature in ["--trace-imports", "--include-types"] {
            let pattern = format!("**/Main.{language}");
            let output = assert_cmd::Command::cargo_bin("context-creator")
                .unwrap()
                .current_dir(fixture.project.path())
                .args([
                    "--include",
                    &pattern,
                    feature,
                    "--quiet",
                    "--style",
                    "paths",
                ])
                .assert()
                .success()
                .get_output()
                .stdout
                .clone();
            let paths = String::from_utf8(output).unwrap();
            assert!(
                paths.contains(fixture.dependency.file_name().unwrap().to_str().unwrap()),
                "{language} {feature}: {paths}"
            );
        }
    }
}

#[test]
fn added_languages_find_callers_and_importing_clients() {
    for language in ["c", "cpp", "cs", "kt", "rb", "php"] {
        let fixture = fixture(language);
        let pattern = format!(
            "**/{}",
            fixture.dependency.file_name().unwrap().to_str().unwrap()
        );
        let output = assert_cmd::Command::cargo_bin("context-creator")
            .unwrap()
            .current_dir(fixture.project.path())
            .args([
                "--include",
                &pattern,
                "--include-callers",
                "--quiet",
                "--style",
                "paths",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let paths = String::from_utf8(output).unwrap();
        assert!(
            paths.contains(fixture.entry.file_name().unwrap().to_str().unwrap()),
            "{language}: {paths}"
        );
    }
}

#[test]
fn modern_module_variants_and_tsx_use_their_actual_grammar() {
    let project = TempDir::new().unwrap();
    for (extension, source) in [
        ("mjs", "import { greet } from './helper.mjs'; export function run() { greet(); }"),
        ("cjs", "const greet = require('./helper.cjs'); function run() { greet(); }"),
        ("mts", "import { greet } from './helper.mjs'; export function run(): void { greet(); }"),
        ("cts", "import { greet } from './helper'; export function run(): void { greet(); }"),
        ("tsx", "import { greet } from './helper'; export function run() { return <section>{greet()}</section>; }"),
        ("pyi", "from typing import Optional\ndef run(value: Optional[str]) -> str: ...\n"),
    ] {
        let path = project.path().join(format!("Main.{extension}"));
        fs::write(&path, source).unwrap();
        let analyzer = get_analyzer_for_file(&path).unwrap().unwrap();
        let result = analyzer.analyze_file(&path, source, &SemanticContext::new(path.clone(), project.path().to_path_buf(), 3)).unwrap();
        assert!(!result.imports.is_empty(), "{extension}");
        assert!(result.exported_functions.iter().any(|function| function.name == "run"), "{extension}: {:?}", result.exported_functions);
        let language = context_creator::core::semantic::grammar_for_file(&path).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language).unwrap();
        assert!(!parser.parse(source, None).unwrap().root_node().has_error(), "{extension}");
    }
}

#[cfg(unix)]
#[test]
fn additional_language_resolvers_reject_existing_symlink_escapes() {
    let outside = TempDir::new().unwrap();
    for language in ["c", "cpp", "cs", "kt", "rb", "php"] {
        let fixture = fixture(language);
        let target = outside.path().join(fixture.dependency.file_name().unwrap());
        fs::write(&target, fs::read(&fixture.dependency).unwrap()).unwrap();
        fs::remove_file(&fixture.dependency).unwrap();
        std::os::unix::fs::symlink(&target, &fixture.dependency).unwrap();
        let resolver = get_resolver_for_file(&fixture.entry).unwrap().unwrap();
        let module = match language {
            "c" => "user.h",
            "cpp" => "user.hpp",
            "cs" => "App.Models.User",
            "kt" => "app.models.User",
            "rb" => "../lib/user",
            "php" => "App\\Models\\User",
            _ => unreachable!(),
        };
        assert!(
            resolver
                .resolve_import(module, &fixture.entry, fixture.project.path())
                .is_err(),
            "{language}"
        );
    }
}

#[test]
fn malformed_sources_recover_without_losing_valid_declarations() {
    for language in ["c", "cpp", "cs", "kt", "rb", "php"] {
        let fixture = fixture(language);
        let source = format!("{}\n@@@ {{{{", fs::read_to_string(&fixture.entry).unwrap());
        let analyzer = get_analyzer_for_file(&fixture.entry).unwrap().unwrap();
        let result = analyzer
            .analyze_file(
                &fixture.entry,
                &source,
                &SemanticContext::new(
                    fixture.entry.clone(),
                    fixture.project.path().to_path_buf(),
                    3,
                ),
            )
            .unwrap();
        assert!(!result.errors.is_empty(), "{language}");
        assert!(
            result
                .exported_functions
                .iter()
                .any(|function| function.name == "run"),
            "{language}"
        );
    }
}

#[test]
fn javascript_module_variants_resolve_extensionless_imports() {
    for extension in ["mjs", "cjs", "mts", "cts"] {
        let project = TempDir::new().unwrap();
        let entry = project.path().join(format!("Main.{extension}"));
        let dependency = project.path().join(format!("helper.{extension}"));
        fs::write(
            &entry,
            "import { greet } from './helper'; export function run() { greet(); }",
        )
        .unwrap();
        fs::write(&dependency, "export function greet() { return 42; }").unwrap();
        let resolver = get_resolver_for_file(&entry).unwrap().unwrap();
        let resolved = resolver
            .resolve_import("./helper", &entry, project.path())
            .unwrap();
        assert_eq!(resolved.path, dependency, "{extension}");
        assert!(!resolved.is_external);
    }
}

#[test]
fn namespace_aliases_and_kotlin_wildcards_resolve_referenced_types() {
    for (language, source, name) in [
        ("cs", "using Customer = App.Models.User;\npublic class Main { public Customer run() { return Customer.create(); } }\n", "Customer"),
        ("kt", "package app\nimport app.models.User as Customer\nfun run(): Customer { return Customer(1) }\n", "Customer"),
        ("kt", "package app\nimport app.models.*\nfun run(): User { return User(1) }\n", "User"),
        ("php", "<?php\nuse App\\Models\\User as Customer;\nfunction run(Customer $user): Customer { return $user; }\n", "Customer"),
    ] {
        let fixture = fixture(language);
        fs::write(&fixture.entry, source).unwrap();
        let analyzer = get_analyzer_for_file(&fixture.entry).unwrap().unwrap();
        let result = analyzer.analyze_file(&fixture.entry, source,
            &SemanticContext::new(fixture.entry.clone(), fixture.project.path().to_path_buf(), 3)).unwrap();
        assert!(result.type_references.iter().any(|reference|
            reference.name == name && reference.definition_path.as_ref() == Some(&fixture.dependency)),
            "{language}: {:?}", result.type_references);
    }
}
