use clap::Parser;
use context_creator::cli::Config;
use context_creator::cli::OutputFormat;
use context_creator::config::ConfigFile;
use context_creator::core::cache::FileCache;
use context_creator::core::context_builder::{generate_digest, generate_markdown, ContextOptions};
use context_creator::core::semantic::analyzer::SemanticContext;
use context_creator::core::semantic::{get_analyzer_for_file, get_resolver_for_file};
use context_creator::core::token::TokenEncoding;
use context_creator::core::walker::{walk_directory, WalkOptions};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn model_and_tokenizer_config_preserve_explicit_cli_values() {
    let config: ConfigFile = toml::from_str(
        r#"
        [defaults]
        model = "provider-default-alias"
        tokenizer = "cl100k-base"
    "#,
    )
    .unwrap();
    let mut defaults = Config::parse_from(["context-creator"]);
    config.apply_to_cli_config(&mut defaults);
    assert_eq!(defaults.model.as_deref(), Some("provider-default-alias"));
    assert_eq!(defaults.get_tokenizer(), TokenEncoding::Cl100kBase);

    let mut explicit = Config::parse_from([
        "context-creator",
        "--model",
        "my-model",
        "--tokenizer",
        "o200k-base",
    ]);
    config.apply_to_cli_config(&mut explicit);
    assert_eq!(explicit.model.as_deref(), Some("my-model"));
    assert_eq!(explicit.get_tokenizer(), TokenEncoding::O200kBase);
}

#[test]
fn shipped_configuration_examples_load() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for name in ["minimal.toml", "comprehensive.toml"] {
        ConfigFile::load_from_file(&root.join("examples/config-examples").join(name)).unwrap();
    }
}

#[test]
fn explicit_logging_flags_override_conflicting_config_defaults() {
    for (flag, defaults) in [("--quiet", "verbose = true"), ("--verbose", "quiet = true")] {
        let config: ConfigFile = toml::from_str(&format!("[defaults]\n{defaults}")).unwrap();
        let mut cli = Config::parse_from(["context-creator", flag]);
        config.apply_to_cli_config(&mut cli);
        cli.validate().unwrap();
        assert_eq!(cli.quiet, flag == "--quiet");
        assert_eq!(cli.verbose > 0, flag == "--verbose");
    }
}

#[test]
fn mcp_caches_distinguish_models_and_tokenizers() {
    use context_creator::mcp_server::{
        cache::{ProcessLocalCacheKey, ProcessRemoteCacheKey},
        ProcessLocalRequest, ProcessRemoteRequest,
    };
    let request: ProcessLocalRequest =
        serde_json::from_value(serde_json::json!({"path": "."})).unwrap();
    let baseline = ProcessLocalCacheKey::from_request(&request);
    let mut changed = request.clone();
    changed.model = Some("another-model".to_string());
    assert_ne!(baseline, ProcessLocalCacheKey::from_request(&changed));
    changed = request;
    changed.tokenizer = Some(TokenEncoding::Cl100kBase);
    assert_ne!(baseline, ProcessLocalCacheKey::from_request(&changed));

    let request: ProcessRemoteRequest =
        serde_json::from_value(serde_json::json!({"repo_url": "https://github.com/example/repo"}))
            .unwrap();
    let baseline = ProcessRemoteCacheKey::from_request(&request);
    let mut changed = request.clone();
    changed.model = Some("another-model".to_string());
    assert_ne!(baseline, ProcessRemoteCacheKey::from_request(&changed));
    changed = request;
    changed.tokenizer = Some(TokenEncoding::Cl100kBase);
    assert_ne!(baseline, ProcessRemoteCacheKey::from_request(&changed));
}

#[test]
fn ollama_accepts_generic_model_and_rejects_conflicting_aliases() {
    let config = Config::parse_from([
        "context-creator",
        "--tool",
        "ollama",
        "--model",
        "custom:latest",
        "--prompt",
        "Explain",
    ]);
    config.validate().unwrap();
    let (command, combined) = config.llm_tool.prepare_command(&config).unwrap();
    assert!(combined);
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        ["run", "custom:latest"]
    );
    let conflict = Config::parse_from([
        "context-creator",
        "--tool",
        "ollama",
        "--model",
        "first",
        "--ollama-model",
        "second",
    ]);
    assert!(conflict.validate().is_err());
}

#[test]
fn explicit_legacy_ollama_model_overrides_generic_config_default() {
    let defaults: ConfigFile = toml::from_str(
        r#"
        [defaults]
        llm_tool = "ollama"
        model = "configured-model"
    "#,
    )
    .unwrap();
    for arguments in [
        vec![
            "context-creator",
            "--ollama-model",
            "explicit-model",
            "--prompt",
            "Explain",
        ],
        vec![
            "context-creator",
            "--tool",
            "ollama",
            "--ollama-model",
            "explicit-model",
            "--prompt",
            "Explain",
        ],
    ] {
        let mut cli = Config::parse_from(arguments);
        defaults.apply_to_cli_config(&mut cli);
        cli.validate().unwrap();
        let (command, _) = cli.llm_tool.prepare_command(&cli).unwrap();
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["run", "explicit-model"]
        );
    }
}

#[test]
fn language_matrix_extracts_imports_definitions_and_calls() {
    let fixtures = [
        ("rs", "use std::io;\npub fn run() { greet(); }\npub fn greet() {}"),
        ("py", "import os\ndef run():\n    greet()\ndef greet():\n    pass\n"),
        ("js", "import { greet } from './helper.js';\nexport function run() { greet(); }"),
        ("jsx", "import { greet } from './helper.js';\nexport function run() { greet(); return <div/>; }"),
        ("ts", "import { greet } from './helper';\nexport function run(): void { greet(); }"),
        ("tsx", "import { greet } from './helper';\nexport function run() { greet(); return <div/>; }"),
        ("go", "package main\nimport \"fmt\"\nfunc Run() { fmt.Println(\"hello\") }"),
        ("swift", "import Foundation\nfunc run() { print(\"hello\") }"),
        ("java", "import java.util.List;\npublic class Main { public void run() { greet(); } public void greet() {} }"),
    ];
    let project = TempDir::new().unwrap();
    for (extension, source) in fixtures {
        let path = project.path().join(format!("Main.{extension}"));
        fs::write(&path, source).unwrap();
        let analyzer = get_analyzer_for_file(&path).unwrap().unwrap();
        let context = SemanticContext::new(path.clone(), project.path().to_path_buf(), 3);
        let analysis = analyzer.analyze_file(&path, source, &context).unwrap();
        assert!(!analysis.imports.is_empty(), "{extension}: missing imports");
        assert!(
            !analysis.exported_functions.is_empty(),
            "{extension}: missing definitions"
        );
        assert!(
            !analysis.function_calls.is_empty(),
            "{extension}: missing calls"
        );
    }
}

fn java_project() -> TempDir {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join("pom.xml"), "<project/>").unwrap();
    let package = project.path().join("src/main/java/example");
    fs::create_dir_all(package.join("models")).unwrap();
    fs::write(
        package.join("models/Customer.java"),
        "package example.models; public class Customer { public String name; }",
    )
    .unwrap();
    fs::write(package.join("Service.java"),
        "package example; import example.models.Customer; public class Service { public Customer load() { return new Customer(); } }").unwrap();
    fs::write(package.join("Main.java"),
        "package example; import example.Service; public class Main { public void run() { new Service().load(); } }").unwrap();
    project
}

#[test]
fn java_extracts_static_imports_wildcards_types_and_visibility() {
    let project = java_project();
    let path = project.path().join("src/main/java/example/Main.java");
    let source = r#"
        package example;
        import example.models.*;
        import static example.Service.load;
        import java.util.List;
        // import fake.Secret;
        public class Main {
            public Customer run(List<Customer> customers) { load(); return new Customer(); }
            private void hidden() {}
        }
    "#;
    fs::write(&path, source).unwrap();
    let analyzer = get_analyzer_for_file(&path).unwrap().unwrap();
    let result = analyzer
        .analyze_file(
            &path,
            source,
            &SemanticContext::new(path.clone(), project.path().to_path_buf(), 3),
        )
        .unwrap();
    assert!(result.errors.is_empty());
    assert!(!result
        .imports
        .iter()
        .any(|import| import.module == "fake.Secret"));
    assert!(result
        .imports
        .iter()
        .any(|import| import.module == "example.Service" && import.items == ["load"]));
    assert!(result
        .imports
        .iter()
        .any(|import| import.module == "example.models.Customer"));
    assert!(result
        .type_references
        .iter()
        .any(|reference| reference.name == "Customer" && reference.definition_path.is_some()));
    assert!(result
        .type_references
        .iter()
        .any(|reference| reference.name == "List" && reference.is_external));
    assert!(result
        .exported_functions
        .iter()
        .any(|function| function.name == "run" && function.is_exported));
    assert!(result
        .exported_functions
        .iter()
        .any(|function| function.name == "hidden" && !function.is_exported));
}

#[test]
fn java_resolves_packages_nested_classes_and_rejects_traversal() {
    let project = java_project();
    let path = project.path().join("src/main/java/example/Main.java");
    let resolver = get_resolver_for_file(&path).unwrap().unwrap();
    for module in ["example.models.Customer", "example.models.Customer.Nested"] {
        let resolved = resolver
            .resolve_import(module, &path, project.path())
            .unwrap();
        assert!(!resolved.is_external);
        assert_eq!(
            resolved.path,
            project
                .path()
                .join("src/main/java/example/models/Customer.java")
                .canonicalize()
                .unwrap()
        );
    }
    for module in [
        "../Secret",
        "example/Secret",
        "example..Secret",
        "example.%2e%2e.Secret",
    ] {
        assert!(resolver
            .resolve_import(module, &path, project.path())
            .is_err());
    }
    assert!(
        resolver
            .resolve_import("java.util.List", &path, project.path())
            .unwrap()
            .is_external
    );
}

#[test]
fn java_cli_traces_transitive_imports_and_type_definitions() {
    let project = java_project();
    for flag in ["--trace-imports", "--include-types"] {
        let entry = if flag == "--include-types" {
            "Service.java"
        } else {
            "Main.java"
        };
        let output = assert_cmd::Command::cargo_bin("context-creator")
            .unwrap()
            .current_dir(project.path())
            .args(["--include", &format!("**/{entry}"), flag, "--quiet"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("public class Customer"), "{flag}: {output}");
        assert!(output.contains("public class Service"), "{flag}: {output}");
    }
}

#[test]
fn caller_expansion_separates_languages_and_java_packages() {
    let project = java_project();
    fs::write(
        project.path().join("unrelated.py"),
        "load() # UNRELATED_PYTHON\n",
    )
    .unwrap();
    let other_package = project.path().join("src/main/java/other");
    fs::create_dir_all(&other_package).unwrap();
    fs::write(
        other_package.join("Other.java"),
        "package other; class Other { void run() { load(); } void load() {} } // UNRELATED_JAVA",
    )
    .unwrap();
    let output = assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args([
            "--include",
            "**/Service.java",
            "--include-callers",
            "--quiet",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("public class Main"), "{output}");
    assert!(!output.contains("UNRELATED_PYTHON"), "{output}");
    assert!(!output.contains("UNRELATED_JAVA"), "{output}");
}

#[test]
fn java_trace_imports_includes_implicit_same_package_types() {
    let project = java_project();
    fs::write(
        project.path().join("src/main/java/example/Main.java"),
        "package example; public class Main { public Service run() { return new Service(); } }",
    )
    .unwrap();
    let output = assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args(["--include", "**/Main.java", "--trace-imports", "--quiet"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("public class Service"), "{output}");
    assert!(output.contains("public class Customer"), "{output}");
}

#[test]
fn quiet_semantic_analysis_suppresses_cycle_warnings() {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join("package.json"), "{}").unwrap();
    fs::write(
        project.path().join("a.js"),
        "import { b } from './b.js'; export function a() { b(); }",
    )
    .unwrap();
    fs::write(
        project.path().join("b.js"),
        "import { a } from './a.js'; export function b() { a(); }",
    )
    .unwrap();
    assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args(["--trace-imports", "--quiet"])
        .assert()
        .success()
        .stderr(predicates::str::is_empty());
}

#[cfg(unix)]
#[test]
fn java_resolver_rejects_symlinks_outside_the_project() {
    let project = java_project();
    let outside = TempDir::new().unwrap();
    fs::write(outside.path().join("Secret.java"), "class Secret {}").unwrap();
    std::os::unix::fs::symlink(outside.path(), project.path().join("escaped")).unwrap();
    let path = project.path().join("src/main/java/example/Main.java");
    let resolver = get_resolver_for_file(&path).unwrap().unwrap();
    assert!(resolver
        .resolve_import("escaped.Secret", &path, project.path())
        .is_err());
}

#[test]
fn markdown_handles_embedded_fences_in_both_renderers() {
    let project = TempDir::new().unwrap();
    let source = "# Example\n\n```rust\nfn main() {}\n```\n\n````\nnested\n````\n";
    fs::write(project.path().join("example.md"), source).unwrap();
    let files = walk_directory(project.path(), WalkOptions::default()).unwrap();
    let cache = Arc::new(FileCache::new());
    let options = ContextOptions::default();
    let outputs = [
        generate_markdown(files.clone(), options.clone(), cache.clone()).unwrap(),
        generate_digest(files, options, cache, OutputFormat::Markdown, ".").unwrap(),
    ];
    for output in outputs {
        assert!(output.contains(&format!("`````markdown\n{source}`````\n")));
    }
}

#[test]
fn xml_escapes_attributes_and_splits_cdata_terminators() {
    let project = TempDir::new().unwrap();
    let source = "before ]]> after & <tag>";
    fs::write(project.path().join("source.txt"), source).unwrap();
    let mut files = walk_directory(project.path(), WalkOptions::default()).unwrap();
    files[0].relative_path = "quote\"&<name>.txt".into();
    let output = generate_digest(
        files,
        ContextOptions::default(),
        Arc::new(FileCache::new()),
        OutputFormat::Xml,
        ".",
    )
    .unwrap();
    assert!(output.contains("path=\"quote&quot;&amp;&lt;name&gt;.txt\""));
    assert!(output.contains("<![CDATA[before ]]]]><![CDATA[> after & <tag>]]>"));
}
