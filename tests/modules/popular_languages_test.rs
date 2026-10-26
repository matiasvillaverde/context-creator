use context_creator::{
    agent::{AgentService, Confidence, ContextRequest},
    core::semantic::{get_analyzer_for_file, symbols::extract_symbols, SemanticContext},
};
use std::{fs, path::Path};
use tempfile::TempDir;

struct Fixture {
    name: &'static str,
    entry: &'static str,
    code: &'static str,
    dependency: &'static str,
    imported: &'static str,
    symbol: Option<&'static str>,
}

#[test]
fn frontend_seed_follows_inline_modules_and_transitive_stylesheets() {
    let project = TempDir::new().unwrap();
    write(project.path(), "index.html", "<!doctype html>\n<HTML><head>\n<LINK REL='stylesheet' HREF='./theme.css'>\n</head><body><section id='session'>Ready</section>\n<script type='module'>\nimport { run } from './app.js';\nrun();\n</script>\n<script type='application/json'>{\"import\": \"./ignored.js\"}</script>\n</body></HTML>\n");
    write(
        project.path(),
        "app.js",
        "import { helper } from './helper.js';\nexport function run() { return helper(); }\n",
    );
    write(
        project.path(),
        "helper.js",
        "export function helper() { return 1; }\n",
    );
    write(
        project.path(),
        "theme.css",
        "@import './base.css';\n.session { color: red; }\n",
    );
    write(project.path(), "base.css", ".base { color: blue; }\n");
    write(project.path(), "ignored.js", "export const unused = 1;\n");
    let mut request = ContextRequest::new(project.path(), "");
    request.seeds.push("index.html".into());
    request.max_tokens = 16_000;
    let response = AgentService::new().build_context(request).unwrap();
    for path in ["index.html", "app.js", "helper.js", "theme.css", "base.css"] {
        assert!(
            response.snippets.iter().any(|snippet| snippet.path == path),
            "missing {path}: {:?}",
            response.gaps
        );
    }
    assert!(response
        .snippets
        .iter()
        .all(|snippet| snippet.path != "ignored.js"));
    assert!(response
        .relations
        .iter()
        .any(|relation| relation.from.path == "index.html"
            && relation.from.line == 6
            && relation.to.path == "app.js"));
}

#[test]
fn commented_discarded_and_computed_loaders_do_not_become_resolved_edges() {
    for (entry, code, ignored) in [
        ("main.sh", "# source ./ignored.sh\nsource \"$MODULE\"\nrun() { echo ready; }\n", "ignored.sh"),
        ("main.clj", "#_(require '[ignored])\n(defn run [] 1)\n", "ignored.clj"),
        ("main.lua", "-- require('ignored')\nlocal dep = require(module_name)\nfunction run() return 1 end\n", "ignored.lua"),
    ] {
        let project = TempDir::new().unwrap();
        write(project.path(), entry, code);
        write(project.path(), ignored, "");
        let path = project.path().join(entry);
        let context = SemanticContext::new(path.clone(), project.path().into(), 5);
        let analysis = get_analyzer_for_file(&path).unwrap().unwrap().analyze_file(&path, code, &context).unwrap();
        assert!(analysis.imports.iter().all(|import| !import.module.contains("ignored")), "{entry}: {:?}", analysis.imports);
        if entry != "main.clj" {
            assert!(analysis.imports.iter().any(|import| import.module == "<computed import>"), "{entry}: {:?}", analysis.imports);
        }
    }
}

#[test]
fn package_imports_and_common_loader_forms_keep_their_local_dependencies() {
    for (entry,code,dependency,imported) in [
        ("main.dart", "import 'package:sample/session.dart';\nint run() { return helper(); }\n", "lib/session.dart", "int helper() { return 1; }\n"),
        ("main.lua", "local dep = require('package')\nfunction run() return dep.helper() end\n", "package/init.lua", "return { helper = function() return 1 end }\n"),
        ("main.exs", "Code.require_file(\"helper.exs\")\ndefmodule Main do\n  def run(), do: Helper.helper()\nend\n", "helper.exs", "defmodule Helper do\n  def helper(), do: 1\nend\n"),
        ("main.ps1", ". .\\Dependency.ps1\nfunction Run { Helper }\n", "Dependency.ps1", "function Helper { return 1 }\n"),
        ("main.ps1", "Import-Module './Dependency.psm1'\nfunction Run { Helper }\n", "Dependency.psm1", "function Helper { return 1 }\n"),
        ("theme.css", "@import url('./base.css');\n.session { color: red; }\n", "base.css", ".base { color: blue; }\n"),
        ("main.pl", "require 'helper.pl';\nsub run { helper(); }\n", "helper.pl", "sub helper { return 1; }\n1;\n"),
    ] {
        let project=TempDir::new().unwrap();
        write(project.path(),entry,code);write(project.path(),dependency,imported);
        write(project.path(),"pubspec.yaml","name: sample\n");
        let mut request=ContextRequest::new(project.path(),"");request.seeds.push(entry.into());request.max_tokens=16000;
        let response=AgentService::new().build_context(request).unwrap();
        assert!(response.snippets.iter().any(|snippet|snippet.path==dependency),"{entry}: {dependency}: {:?}",response.gaps);
    }
}

#[test]
fn sql_links_are_inferred_and_ambiguous_or_case_distinct_objects_stay_unresolved() {
    let project = TempDir::new().unwrap();
    write(project.path(), "consumer.sql", "SELECT id FROM sessions;\n");
    write(
        project.path(),
        "schema.sql",
        "CREATE TABLE sessions (id INTEGER);\n",
    );
    let mut request = ContextRequest::new(project.path(), "");
    request.seeds.push("consumer.sql".into());
    let service = AgentService::new();
    let matched = service.build_context(request.clone()).unwrap();
    assert!(matched
        .relations
        .iter()
        .any(|relation| relation.to.path == "schema.sql"
            && relation.confidence == Confidence::Inferred));
    write(
        project.path(),
        "other.sql",
        "CREATE TABLE sessions (id INTEGER);\n",
    );
    let ambiguous = service.build_context(request.clone()).unwrap();
    assert!(ambiguous
        .snippets
        .iter()
        .all(|snippet| snippet.path == "consumer.sql"));
    assert!(ambiguous
        .gaps
        .iter()
        .any(|gap| gap.kind == "unresolved_import"));
    fs::remove_file(project.path().join("other.sql")).unwrap();
    write(
        project.path(),
        "schema.sql",
        "CREATE TABLE \"Sessions\" (id INTEGER);\n",
    );
    let different = service.build_context(request).unwrap();
    assert!(different
        .snippets
        .iter()
        .all(|snippet| snippet.path == "consumer.sql"));
}

#[test]
fn terraform_module_reads_include_sibling_files_without_a_dense_relationship_clique() {
    let project = TempDir::new().unwrap();
    for index in 0..25 {
        write(
            project.path(),
            &format!("module/item_{index}.tf"),
            &format!("variable \"item_{index}\" {{ default = {index} }}\n"),
        );
    }
    let mut request = ContextRequest::new(project.path(), "");
    request.seeds.push("module/item_24.tf".into());
    request.max_tokens = 16000;
    let response = AgentService::new().build_context(request).unwrap();
    assert_eq!(
        response
            .snippets
            .iter()
            .map(|snippet| &snippet.path)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        25
    );
    assert!(
        response
            .relations
            .iter()
            .filter(|relation| relation.kind == "dependency")
            .count()
            <= 48
    );
}

#[test]
fn go_imports_include_all_local_package_source_files() {
    let project = TempDir::new().unwrap();
    write(
        project.path(),
        "go.mod",
        "module example.local/app\n\ngo 1.22\n",
    );
    write(
        project.path(),
        "main.go",
        "package main\nimport \"example.local/app/helpers\"\nfunc main() { helpers.Run() }\n",
    );
    write(
        project.path(),
        "helpers/a.go",
        "package helpers\nfunc Run() int { return helper() }\n",
    );
    write(
        project.path(),
        "helpers/b.go",
        "package helpers\nfunc helper() int { return 42 }\n",
    );
    let mut request = ContextRequest::new(project.path(), "");
    request.seeds.push("main.go".into());
    let response = AgentService::new().build_context(request).unwrap();
    for path in ["helpers/a.go", "helpers/b.go"] {
        assert!(
            response.snippets.iter().any(|snippet| snippet.path == path),
            "missing {path}"
        );
    }
}

#[test]
fn short_form_julia_and_html_ids_have_exact_source_declarations() {
    for (path, code, symbol) in [
        ("main.jl", "run(value) = value + 1\n", "run"),
        (
            "index.html",
            "<section id=\"session-panel\"><p>Session status</p></section>\n",
            "session-panel",
        ),
    ] {
        let project = TempDir::new().unwrap();
        write(project.path(), path, code);
        let symbols = extract_symbols(&project.path().join(path), code).unwrap();
        assert!(
            symbols.iter().any(|found| found.name == symbol),
            "{path}: {symbols:?}"
        );
        let mut request = ContextRequest::new(project.path(), "");
        request.seeds.push(path.into());
        let response = AgentService::new().build_context(request).unwrap();
        assert!(response.snippets.iter().any(|snippet| snippet
            .content
            .as_ref()
            .is_some_and(|content| code.contains(content))));
    }
}

#[cfg(unix)]
#[test]
fn dart_package_resolution_does_not_read_a_symlinked_manifest() {
    use context_creator::core::semantic::get_resolver_for_file;
    let project = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    write(project.path(), "main.dart", "void main() {}\n");
    write(project.path(), "lib/item.dart", "class Item {}\n");
    write(outside.path(), "pubspec.yaml", "name: sample\n");
    std::os::unix::fs::symlink(
        outside.path().join("pubspec.yaml"),
        project.path().join("pubspec.yaml"),
    )
    .unwrap();
    let resolver = get_resolver_for_file(&project.path().join("main.dart"))
        .unwrap()
        .unwrap();
    assert!(
        resolver
            .resolve_import(
                "package:sample/item.dart",
                &project.path().join("main.dart"),
                project.path()
            )
            .unwrap()
            .is_external
    );
}
fn fixtures() -> Vec<Fixture> {
    vec![
    Fixture{name:"Bash",entry:"main.sh",code:"source './dependency.sh'\nrun() { helper; }\n",dependency:"dependency.sh",imported:"helper() { echo ready; }\n",symbol:Some("run")},
    Fixture{name:"Dart",entry:"main.dart",code:"import 'dependency.dart';\nint run() { return helper(); }\n",dependency:"dependency.dart",imported:"int helper() { return 1; }\n",symbol:Some("run")},
    Fixture{name:"Lua",entry:"main.lua",code:"local dep = require('dependency')\nfunction run() return dep.helper() end\n",dependency:"dependency.lua",imported:"local M = {}\nfunction M.helper() return 1 end\nreturn M\n",symbol:Some("run")},
    Fixture{name:"Scala",entry:"Main.scala",code:"import app.Helper\nobject Main { def run(): Int = Helper.helper() }\n",dependency:"src/main/scala/app/Helper.scala",imported:"package app\nobject Helper { def helper(): Int = 1 }\n",symbol:Some("run")},
    Fixture{name:"Elixir",entry:"main.ex",code:"defmodule Main do\n  alias Helper\n  def run(), do: Helper.helper()\nend\n",dependency:"lib/helper.ex",imported:"defmodule Helper do\n  def helper(), do: 1\nend\n",symbol:Some("run")},
    Fixture{name:"R",entry:"main.R",code:"source('dependency.R')\nrun <- function() { helper() }\n",dependency:"dependency.R",imported:"helper <- function() { 1 }\n",symbol:Some("run")},
    Fixture{name:"Julia",entry:"main.jl",code:"include(\"dependency.jl\")\nfunction run()\n    helper()\nend\n",dependency:"dependency.jl",imported:"function helper()\n    return 1\nend\n",symbol:Some("run")},
    Fixture{name:"Haskell",entry:"Main.hs",code:"module Main where\nimport Helper\nrun x = helper x\n",dependency:"src/Helper.hs",imported:"module Helper where\nhelper x = x + 1\n",symbol:Some("run")},
    Fixture{name:"Clojure",entry:"main.clj",code:"(ns main (:require [helper :as h]))\n(defn run [] (h/help))\n",dependency:"src/helper.clj",imported:"(ns helper)\n(defn help [] 1)\n",symbol:Some("run")},
    Fixture{name:"PowerShell",entry:"main.ps1",code:". ./dependency.ps1\nfunction Run { Helper }\n",dependency:"dependency.ps1",imported:"function Helper { return 1 }\n",symbol:Some("Run")},
    Fixture{name:"Perl",entry:"main.pl",code:"use Helper;\nsub run { Helper::helper(); }\n",dependency:"lib/Helper.pm",imported:"package Helper;\nsub helper { return 1; }\n1;\n",symbol:Some("run")},
    Fixture{name:"HCL",entry:"main.tf",code:"module \"network\" {\n  source = \"./modules/network\"\n}\n",dependency:"modules/network/main.tf",imported:"resource \"network\" \"core\" {}\n",symbol:Some("module.network")},
    Fixture{name:"SQL",entry:"consumer.sql",code:"SELECT id FROM sessions;\n",dependency:"schema.sql",imported:"CREATE TABLE sessions (id INTEGER);\n",symbol:None},
    Fixture{name:"HTML",entry:"index.html",code:"<!doctype html><html><head><link rel=\"stylesheet\" href=\"theme.css\"></head><body><script src=\"app.js\"></script></body></html>\n",dependency:"app.js",imported:"export function run() { return 1; }\n",symbol:None},
    Fixture{name:"CSS",entry:"theme.css",code:"@import './base.css';\n.session { color: red; }\n",dependency:"base.css",imported:".base { color: blue; }\n",symbol:Some(".session")},
]
}
fn write(root: &Path, path: &str, code: &str) {
    let target = root.join(path);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, code).unwrap();
}

#[test]
fn popular_languages_parse_real_declarations_without_placeholder_analyzers() {
    let mut failures = Vec::new();
    for fixture in fixtures() {
        let project = TempDir::new().unwrap();
        write(project.path(), fixture.entry, fixture.code);
        write(project.path(), fixture.dependency, fixture.imported);
        let path = project.path().join(fixture.entry);
        let Some(analyzer) = get_analyzer_for_file(&path).unwrap() else {
            failures.push(format!("{} analyzer missing", fixture.name));
            continue;
        };
        match analyzer.analyze_file(
            &path,
            fixture.code,
            &SemanticContext::new(path.clone(), project.path().into(), 5),
        ) {
            Ok(result) => {
                if !result.errors.is_empty() {
                    failures.push(format!(
                        "{} parse errors: {:?}",
                        fixture.name, result.errors
                    ));
                }
            }
            Err(error) => {
                failures.push(format!("{} analysis: {error}", fixture.name));
                continue;
            }
        }
        if let Some(symbol) = fixture.symbol {
            match extract_symbols(&path, fixture.code) {
                Ok(symbols) if symbols.iter().any(|found| found.name == symbol) => {}
                other => failures.push(format!(
                    "{} missing declaration {symbol}: {other:?}",
                    fixture.name
                )),
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn popular_languages_follow_local_dependencies_into_agent_context() {
    let mut failures = Vec::new();
    for fixture in fixtures() {
        let project = TempDir::new().unwrap();
        write(project.path(), fixture.entry, fixture.code);
        write(project.path(), fixture.dependency, fixture.imported);
        if fixture.name == "HTML" {
            write(project.path(), "theme.css", ".session { color: red; }\n");
        }
        let mut request = ContextRequest::new(project.path(), "");
        request.seeds.push(fixture.entry.into());
        request.max_tokens = 16000;
        match AgentService::new().build_context(request) {
            Ok(response) => {
                if !response
                    .snippets
                    .iter()
                    .any(|snippet| snippet.path == fixture.dependency)
                {
                    failures.push(format!(
                        "{} missing {}: {:?}",
                        fixture.name, fixture.dependency, response.gaps
                    ));
                }
            }
            Err(error) => failures.push(format!("{} context: {error}", fixture.name)),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
