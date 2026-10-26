use context_creator::{
    agent::{AgentService, Confidence, ContextRequest, ContextResponse, Representation},
    core::{
        semantic::{get_analyzer_for_file, get_resolver_for_file, SemanticContext},
        token::TokenCounter,
    },
    mcp_server::{progressive::read_file_sync, ReadFileRequest},
};
use std::{collections::BTreeSet, fs, path::Path};
use tempfile::TempDir;

fn write(root: &Path, path: &str, source: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, source).unwrap();
}

fn chain(root: &Path, language: &str, count: usize) {
    for index in 0..count {
        let next = (index + 1 < count).then(|| index + 1);
        let source = match language {
            "js" => match next {
                Some(next) => format!("// dependency {index}\nimport {{ step{next} }} from './level{next}.js';\nexport function step{index}() {{ return step{next}(); }}\n"),
                None => format!("export function step{index}() {{ return 1; }}\n"),
            },
            "zig" => match next {
                Some(next) => format!("// dependency {index}\nconst next = @import(\"level{next}.zig\");\npub fn step{index}() u32 {{ return next.step{next}(); }}\n"),
                None => format!("pub fn step{index}() u32 {{ return 1; }}\n"),
            },
            "py" => match next {
                Some(next) => format!("# dependency {index}\nfrom .level{next} import step{next}\ndef step{index}():\n    return step{next}()\n"),
                None => format!("def step{index}(): return 1\n"),
            },
            "rs" => match next {
                Some(next) => format!("// dependency {index}\nuse crate::level{next};\npub fn step{index}() -> u32 {{ level{next}::step{next}() }}\n"),
                None => format!("pub fn step{index}() -> u32 {{ 1 }}\n"),
            },
            _ => unreachable!(),
        };
        write(root, &format!("src/level{index}.{language}"), &source);
    }
}

fn paths(response: &ContextResponse) -> BTreeSet<&str> {
    response
        .snippets
        .iter()
        .map(|snippet| snippet.path.as_str())
        .collect()
}

#[test]
fn primary_import_chain_survives_large_caller_fanout() {
    let project = TempDir::new().unwrap();
    chain(project.path(), "js", 9);
    for index in 0..260 {
        write(project.path(), &format!("src/client{index}.js"), &format!(
            "import {{ step0 }} from './level0.js';\nexport function client{index}() {{ return step0(); }}\n"
        ));
    }
    let mut request = ContextRequest::new(project.path(), "");
    request.seeds.push("src/level0.js".into());
    request.max_tokens = 65_536;
    let response = AgentService::new().build_context(request).unwrap();
    let selected = paths(&response);
    for index in 0..9 {
        assert!(
            selected.contains(format!("src/level{index}.js").as_str()),
            "primary dependency {index} lost to callers: {selected:?}"
        );
    }
    assert!(response.gaps.iter().any(|gap| gap.kind == "graph_limit"));
    assert!(response
        .gaps
        .iter()
        .any(|gap| gap.kind == "candidate_limit"));
}

#[test]
fn zig_extracts_imports_calls_types_and_complete_declarations() {
    let project = TempDir::new().unwrap();
    write(
        project.path(),
        "src/types.zig",
        "pub const Session = struct { ttl: u64 = 3600, };\n",
    );
    let source = "const types = @import(\"types.zig\");\nconst std = @import(\"std\");\npub fn expired(session: types.Session, now: u64) bool { return now > session.ttl; }\npub fn run() bool { return expired(.{}, 42); }\ntest \"expiration\" { try std.testing.expect(run()); }\n";
    write(project.path(), "src/main.zig", source);
    let path = project.path().join("src/main.zig");
    let context = SemanticContext::new(path.clone(), project.path().into(), 5);
    let analyzer = get_analyzer_for_file(&path)
        .unwrap()
        .expect("Zig analyzer missing");
    let analysis = analyzer.analyze_file(&path, source, &context).unwrap();
    assert!(analysis.errors.is_empty(), "{:?}", analysis.errors);
    assert!(analysis
        .imports
        .iter()
        .any(|import| import.module == "types.zig" && import.items.contains(&"types".into())));
    assert!(analysis
        .exported_functions
        .iter()
        .any(|function| function.name == "expired" && function.is_exported));
    assert!(analysis
        .function_calls
        .iter()
        .any(|call| call.name == "expired"));
    assert!(analysis
        .type_references
        .iter()
        .any(|reference| reference.name == "Session"
            && reference.definition_path.as_deref()
                == Some(
                    project
                        .path()
                        .join("src/types.zig")
                        .canonicalize()
                        .unwrap()
                        .as_path()
                )));
    let mut request = ContextRequest::new(project.path(), "");
    request.seeds.push("src/main.zig".into());
    let response = AgentService::new().build_context(request).unwrap();
    assert!(paths(&response).contains("src/types.zig"));
    assert!(response
        .snippets
        .iter()
        .any(|snippet| snippet.symbol.as_deref() == Some("expired")
            && snippet.representation == Representation::Full));
    assert!(response
        .snippets
        .iter()
        .any(|snippet| snippet.kind == "type"
            && snippet.symbol.as_deref() == Some("Session")
            && snippet
                .content
                .as_ref()
                .is_some_and(|source| source.contains("ttl: u64 = 3600"))));
    assert!(response
        .snippets
        .iter()
        .any(|snippet| snippet.kind == "test"
            && snippet
                .content
                .as_ref()
                .is_some_and(|source| source.contains("std.testing.expect(run())"))));
}

#[test]
fn zig_resolves_file_imports_without_guessing_build_modules_or_escaping_root() {
    let project = TempDir::new().unwrap();
    write(project.path(), "src/main.zig", "pub fn main() void {}\n");
    write(
        project.path(),
        "types.zig",
        "pub const Session = struct {};\n",
    );
    write(project.path(), "std.zig", "pub fn pretend() void {}\n");
    let from = project.path().join("src/main.zig");
    let resolver = get_resolver_for_file(&from)
        .unwrap()
        .expect("Zig resolver missing");
    let local = resolver
        .resolve_import("../types.zig", &from, project.path())
        .unwrap();
    assert_eq!(
        local.path,
        project.path().join("types.zig").canonicalize().unwrap()
    );
    assert!(!local.is_external);
    for module in ["std", "builtin", "root", "unconfigured_package"] {
        assert!(
            resolver
                .resolve_import(module, &from, project.path())
                .unwrap()
                .is_external,
            "{module}"
        );
    }
    for module in [
        "../../outside.zig",
        "../%2e%2e/escape.zig",
        "C:/escape.zig",
        "missing.zig",
    ] {
        assert!(
            resolver
                .resolve_import(module, &from, project.path())
                .is_err(),
            "{module}"
        );
    }
}

#[test]
fn file_seed_follows_the_entire_local_import_chain_beyond_recipe_depth() {
    for language in ["js", "zig", "py", "rs"] {
        let project = TempDir::new().unwrap();
        chain(project.path(), language, 9);
        write(
            project.path(),
            &format!("src/unrelated.{language}"),
            "// unrelated source\n",
        );
        let mut request = ContextRequest::new(project.path(), "");
        request.seeds.push(format!("src/level0.{language}").into());
        request.max_tokens = 16000;
        let response = AgentService::new().build_context(request).unwrap();
        let included = paths(&response);
        for index in 0..9 {
            assert!(
                included.contains(format!("src/level{index}.{language}").as_str()),
                "{language}: missing level {index}"
            );
        }
        assert!(!included.contains(format!("src/unrelated.{language}").as_str()));
        for index in 0..8 {
            assert!(
                response
                    .relations
                    .iter()
                    .any(|relation| relation.kind == "dependency"
                        && relation.from.path == format!("src/level{index}.{language}")
                        && relation.to.path == format!("src/level{}.{language}", index + 1)
                        && relation.from.line == 2
                        && relation.confidence == Confidence::Resolved),
                "{language}: missing located edge {index}"
            );
        }
        assert_eq!(
            response.token_count,
            TokenCounter::new()
                .unwrap()
                .count_tokens(&serde_json::to_string(&response).unwrap())
                .unwrap()
        );
        assert!(response.token_count <= 16000);
    }
}

#[test]
fn zig_keeps_comments_out_of_imports_and_reports_computed_and_external_sources() {
    let project = TempDir::new().unwrap();
    write(project.path(), "main.zig", "// @import(\"fake.zig\")\nconst std = @import(\"std\");\nconst package = @import(\"generated_module\");\nconst dynamic = @import(module_name);\nconst missing = @import(\"missing.zig\");\npub fn run() void {}\n");
    let mut request = ContextRequest::new(project.path(), "");
    request.seeds.push("main.zig".into());
    request.max_tokens = 16000;
    let response = AgentService::new().build_context(request).unwrap();
    assert!(response.gaps.iter().any(|gap| gap.kind == "external_import"
        && gap.line == Some(2)
        && gap.detail.contains("std")));
    assert!(response.gaps.iter().any(|gap| gap.kind == "external_import"
        && gap.line == Some(3)
        && gap.detail.contains("generated_module")));
    for line in [4, 5] {
        assert!(
            response
                .gaps
                .iter()
                .any(|gap| gap.kind == "unresolved_import" && gap.line == Some(line)),
            "missing gap at line {line}"
        );
    }
    assert!(!response
        .gaps
        .iter()
        .any(|gap| gap.detail.contains("fake.zig")));
}

#[test]
fn zig_embedded_resources_and_build_configuration_are_available_as_context() {
    let project = TempDir::new().unwrap();
    write(project.path(), "src/main.zig", "pub const template = @embedFile(\"template\");\npub fn run() usize { return template.len; }\n");
    write(
        project.path(),
        "src/template",
        "Hello from a bundled template\n",
    );
    write(
        project.path(),
        "build.zig",
        "const std = @import(\"std\");\npub fn build(b: *std.Build) void { _ = b; }\n",
    );
    write(
        project.path(),
        "build.zig.zon",
        ".{ .name = .sample, .version = \"0.0.0\" }\n",
    );
    let mut request = ContextRequest::new(project.path(), "");
    request.seeds.push("src/main.zig".into());
    let response = AgentService::new().build_context(request).unwrap();
    let selected = paths(&response);
    for path in ["src/template", "build.zig", "build.zig.zon"] {
        assert!(selected.contains(path), "missing {path}");
    }
}

#[test]
fn ignored_dependencies_are_reported_without_reopening_their_source() {
    let project = TempDir::new().unwrap();
    chain(project.path(), "zig", 7);
    write(project.path(), ".contextignore", "src/level4.zig\n");
    let mut request = ContextRequest::new(project.path(), "");
    request.seeds.push("src/level0.zig".into());
    let response = AgentService::new().build_context(request).unwrap();
    assert!(paths(&response).contains("src/level3.zig"));
    assert!(!paths(&response).contains("src/level4.zig"));
    assert!(!paths(&response).contains("src/level5.zig"));
    assert!(response
        .gaps
        .iter()
        .any(|gap| gap.kind == "unresolved_import"
            && gap.path.as_deref() == Some("src/level3.zig")
            && gap.detail.contains("level4.zig")));
}

#[test]
fn constrained_dependency_context_reports_omissions_and_matches_its_token_count() {
    let project = TempDir::new().unwrap();
    chain(project.path(), "zig", 14);
    let request: ReadFileRequest = serde_json::from_value(serde_json::json!({"root":project.path(), "path":"src/level0.zig", "dependency_tokens":1200})).unwrap();
    let response = read_file_sync(request).unwrap();
    let value = serde_json::to_value(response).unwrap();
    let context: ContextResponse =
        serde_json::from_value(value["dependency_context"].clone()).unwrap();
    assert!(context.omitted_snippets > 0);
    assert!(context.token_count <= 1200);
    assert_eq!(
        context.token_count,
        TokenCounter::new()
            .unwrap()
            .count_tokens(&serde_json::to_string(&context).unwrap())
            .unwrap()
    );
    assert!(!context.omissions.is_empty() || context.truncated_metadata > 0);
}

#[cfg(unix)]
#[test]
fn zig_resolver_denies_inside_and_outside_symlink_imports() {
    let project = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    write(project.path(), "src/main.zig", "pub fn run() void {}\n");
    write(project.path(), "real.zig", "pub fn real() void {}\n");
    write(outside.path(), "outside.zig", "pub fn outside() void {}\n");
    std::os::unix::fs::symlink(
        project.path().join("real.zig"),
        project.path().join("alias.zig"),
    )
    .unwrap();
    std::os::unix::fs::symlink(outside.path(), project.path().join("outside")).unwrap();
    let from = project.path().join("src/main.zig");
    let resolver = get_resolver_for_file(&from).unwrap().unwrap();
    for import in ["../alias.zig", "../outside/outside.zig"] {
        assert!(
            resolver
                .resolve_import(import, &from, project.path())
                .is_err(),
            "{import}"
        );
    }
}

#[test]
fn dependency_cycles_terminate_and_retain_the_cycle_edge() {
    let project = TempDir::new().unwrap();
    chain(project.path(), "js", 8);
    write(
        project.path(),
        "src/level7.js",
        "import { step0 } from './level0.js';\nexport function step7() { return 1; }\n",
    );
    let mut request = ContextRequest::new(project.path(), "");
    request.seeds.push("src/level0.js".into());
    request.max_tokens = 16000;
    let response = AgentService::new().build_context(request).unwrap();
    assert_eq!(paths(&response).len(), 8);
    let edges = response
        .relations
        .iter()
        .filter(|relation| relation.kind == "dependency")
        .map(|relation| (&relation.from.path, &relation.to.path))
        .collect::<BTreeSet<_>>();
    assert_eq!(edges.len(), 8);
    assert!(edges
        .iter()
        .any(|(from, to)| from.as_str() == "src/level7.js" && to.as_str() == "src/level0.js"));
}

#[test]
fn read_file_can_include_transitive_context_under_a_separate_budget() {
    let project = TempDir::new().unwrap();
    chain(project.path(), "js", 7);
    let request: ReadFileRequest = serde_json::from_value(serde_json::json!({
        "root":project.path(),"path":"src/level0.js","max_lines":1,"max_tokens":30,"dependency_tokens":16000
    })).unwrap();
    let response = read_file_sync(request).unwrap();
    assert_eq!(response.content, "// dependency 0\n");
    assert_eq!(response.next_line, Some(2));
    assert!(response.token_count <= 30);
    let value = serde_json::to_value(response).unwrap();
    let context: ContextResponse = serde_json::from_value(
        value
            .get("dependency_context")
            .expect("dependency context missing")
            .clone(),
    )
    .unwrap();
    assert!(paths(&context).contains("src/level6.js"));
    assert!(context.token_count <= 16000);
}

#[test]
fn zig_cli_expands_transitive_imports_and_detects_its_project_root() {
    let project = TempDir::new().unwrap();
    chain(project.path(), "zig", 4);
    write(
        project.path(),
        "build.zig",
        "const std = @import(\"std\");\npub fn build(b: *std.Build) void { _ = b; }\n",
    );
    write(project.path(), ".context-creator.toml", "[defaults]\n");
    let output = assert_cmd::Command::cargo_bin("context-creator")
        .unwrap()
        .current_dir(project.path())
        .args([
            "src",
            "--include",
            "**/level0.zig",
            "--trace-imports",
            "--style",
            "paths",
            "--quiet",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();
    for index in 0..4 {
        assert!(
            output.contains(&format!("level{index}.zig")),
            "missing level {index}: {output}"
        );
    }
}

#[test]
fn zig_containers_keep_fields_methods_and_local_type_bindings() {
    let project = TempDir::new().unwrap();
    let code = "pub const State = enum { active, expired };\npub const Session = struct {\n    state: State = .active,\n    ttl: u64 = 3600,\n    pub fn expired(self: @This(), now: u64) bool { return now > self.ttl; }\n};\npub fn inspect(session: Session) State { return session.state; }\n";
    write(project.path(), "main.zig", code);
    let path = project.path().join("main.zig");
    let analyzer = get_analyzer_for_file(&path).unwrap().unwrap();
    let analysis = analyzer
        .analyze_file(
            &path,
            code,
            &SemanticContext::new(path.clone(), project.path().into(), 5),
        )
        .unwrap();
    assert!(analysis.errors.is_empty());
    for name in ["Session", "State"] {
        assert!(
            analysis
                .type_references
                .iter()
                .any(|reference| reference.name == name
                    && reference.definition_path == Some(path.canonicalize().unwrap())),
            "{name}"
        );
    }
    let mut request = ContextRequest::new(project.path(), "");
    request.seeds.push("main.zig".into());
    let response = AgentService::new().build_context(request).unwrap();
    let class = response
        .snippets
        .iter()
        .find(|snippet| snippet.symbol.as_deref() == Some("Session"))
        .unwrap();
    assert_eq!(class.representation, Representation::Full);
    assert!(class.content.as_ref().unwrap().contains("ttl: u64 = 3600"));
    let combined = response
        .snippets
        .iter()
        .filter_map(|snippet| snippet.content.as_deref())
        .collect::<String>();
    assert_eq!(combined.matches("return now > self.ttl").count(), 1);
    assert!(!response
        .gaps
        .iter()
        .any(|gap| gap.kind == "unresolved_type"));
}

#[test]
fn graph_and_candidate_limits_are_visible_in_deep_dependency_context() {
    let project = TempDir::new().unwrap();
    chain(project.path(), "zig", 260);
    let mut request = ContextRequest::new(project.path(), "");
    request.seeds.push("src/level0.zig".into());
    request.max_tokens = 16000;
    let response = AgentService::new().build_context(request).unwrap();
    for kind in ["graph_limit", "candidate_limit"] {
        assert!(
            response.gaps.iter().any(|gap| gap.kind == kind),
            "missing {kind}"
        );
    }
    assert!(response.omitted_snippets > 0);
    assert!(response.token_count <= 16000);
}

#[test]
fn malformed_zig_still_returns_recoverable_declarations_with_a_parse_gap() {
    let project = TempDir::new().unwrap();
    write(
        project.path(),
        "main.zig",
        "pub fn valid() u32 { return 42; }\npub const Broken = struct { field:\n",
    );
    let mut request = ContextRequest::new(project.path(), "");
    request.seeds.push("main.zig".into());
    let response = AgentService::new().build_context(request).unwrap();
    assert!(response
        .snippets
        .iter()
        .any(|snippet| snippet.symbol.as_deref() == Some("valid")));
    assert!(response.gaps.iter().any(|gap| gap.kind == "parse_error"));
}
