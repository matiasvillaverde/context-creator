use context_creator::core::cache::FileCache;
use context_creator::core::token::TokenCounter;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn map_retains_signatures_and_locations_without_function_bodies() {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join("main.rs"), "pub struct Customer { pub id: u64 }\n\npub fn authenticate(token: &str) -> bool {\n    let implementation_only = token.len();\n    implementation_only > 10\n}\n").unwrap();
    let output = assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args(["--style", "map", "--quiet"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let map = String::from_utf8(output).unwrap();
    assert!(map.starts_with("# Repository map\n"));
    assert!(map.contains("## main.rs"));
    assert!(map.contains("1: pub struct Customer"), "{map}");
    assert!(
        map.contains("3: pub fn authenticate(token: &str) -> bool"),
        "{map}"
    );
    assert!(!map.contains("implementation_only"), "{map}");
    assert!(TokenCounter::new().unwrap().count_tokens(&map).unwrap() <= 4096);
}

#[test]
fn maps_compact_large_implementations_and_obey_explicit_budgets() {
    let project = TempDir::new().unwrap();
    fs::write(
        project.path().join("large.py"),
        format!(
            "def process_customer(customer: str):\n{}    return customer\n",
            "    # implementation detail\n".repeat(1500)
        ),
    )
    .unwrap();
    let output = assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args(["--style", "map", "--max-tokens", "200", "--quiet"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let map = String::from_utf8(output).unwrap();
    assert!(map.contains("process_customer(customer: str):"), "{map}");
    assert!(!map.contains("implementation detail"));
    assert!(TokenCounter::new().unwrap().count_tokens(&map).unwrap() <= 200);
}

#[test]
fn declaration_cache_reuses_the_loaded_content_snapshot() {
    let project = TempDir::new().unwrap();
    let path = project.path().join("main.rs");
    fs::write(&path, "pub fn first() {}").unwrap();
    let cache = FileCache::new();
    let first = cache.get_symbols(&path).unwrap();
    let second = cache.get_symbols(&path).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(first[0].name, "first");
}

#[test]
fn dependency_ranking_places_shared_definitions_ahead_of_unrelated_files() {
    let project = TempDir::new().unwrap();
    fs::create_dir(project.path().join(".git")).unwrap();
    fs::write(
        project.path().join("a_unrelated.py"),
        "def unrelated(): pass\n",
    )
    .unwrap();
    fs::write(
        project.path().join("z_shared.py"),
        "def authenticate(): return True\n",
    )
    .unwrap();
    for file in ["client.py", "service.py"] {
        fs::write(
            project.path().join(file),
            "from z_shared import authenticate\ndef run(): return authenticate()\n",
        )
        .unwrap();
    }
    let output = assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args(["--style", "map", "--quiet"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let map = String::from_utf8(output).unwrap();
    assert!(
        map.find("## z_shared.py").unwrap() < map.find("## a_unrelated.py").unwrap(),
        "{map}"
    );
}

#[test]
fn cpp_declarations_in_h_headers_use_the_cpp_grammar() {
    let symbols = context_creator::core::semantic::symbols::extract_symbols(
        std::path::Path::new("user.h"),
        "namespace app { class User { public: int id; int name() { return 123; } }; }",
    )
    .unwrap();
    assert!(symbols.iter().any(|symbol| symbol.name == "User"));
    assert!(symbols.iter().any(|symbol| symbol.name == "name"));
    assert!(!symbols
        .iter()
        .any(|symbol| symbol.signature.contains("return 123")));
}

#[test]
fn maps_omit_arrow_and_function_expression_bodies() {
    let source = "export const first = (value) => { const implementation_only = value; return implementation_only; };\nconst second = function(value) { return value * 2; };\nexport const third = (value) => value + 1;\n";
    for extension in ["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"] {
        let path = std::path::PathBuf::from(format!("main.{extension}"));
        let symbols =
            context_creator::core::semantic::symbols::extract_symbols(&path, source).unwrap();
        for name in ["first", "second", "third"] {
            let symbol = symbols.iter().find(|symbol| symbol.name == name).unwrap();
            assert!(
                !symbol.signature.contains("implementation_only"),
                "{extension}: {symbol:?}"
            );
            assert!(
                !symbol.signature.contains("return value"),
                "{extension}: {symbol:?}"
            );
            assert!(
                !symbol.signature.contains("value + 1"),
                "{extension}: {symbol:?}"
            );
        }
        assert!(symbols
            .iter()
            .find(|symbol| symbol.name == "third")
            .unwrap()
            .signature
            .ends_with("=>"));
    }
}
