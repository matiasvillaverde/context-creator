use clap::Parser;
use context_creator::cli::{Config, OutputFormat};
use context_creator::core::cache::FileCache;
use context_creator::core::context_builder::{
    generate_digest_with_report, generate_markdown, ContextOptions,
};
use context_creator::core::token::{TokenCounter, TokenEncoding};
use context_creator::core::walker::{walk_directory, WalkOptions};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn rendered_budgets_include_formatting_and_keep_complete_files() {
    let root = TempDir::new().unwrap();
    for index in 0..20 {
        fs::write(
            root.path().join(format!("file_{index}.md")),
            format!(
                "BEGIN_{index}\n```\n{}\n]]>\nEND_{index}\n",
                "unicode 日本語 & <tag> ".repeat(35)
            ),
        )
        .unwrap();
    }
    let files = walk_directory(root.path(), WalkOptions::default()).unwrap();
    for encoding in [TokenEncoding::Cl100kBase, TokenEncoding::O200kBase] {
        let counter = TokenCounter::with_encoding(encoding).unwrap();
        let options = ContextOptions {
            max_tokens: Some(1200),
            tokenizer: encoding,
            enhanced_context: true,
            ..Default::default()
        };
        for style in [
            OutputFormat::Markdown,
            OutputFormat::Xml,
            OutputFormat::Plain,
            OutputFormat::Paths,
        ] {
            let result = generate_digest_with_report(
                files.clone(),
                options.clone(),
                Arc::new(FileCache::new()),
                style,
                "long & <quoted> directory",
            )
            .unwrap();
            assert!(
                counter.count_tokens(&result.text).unwrap() <= 1200,
                "{style:?}"
            );
            assert_eq!(
                result.token_count,
                Some(counter.count_tokens(&result.text).unwrap())
            );
            if style != OutputFormat::Paths {
                for index in 0..20 {
                    assert_eq!(
                        result.text.contains(&format!("BEGIN_{index}\n")),
                        result.text.contains(&format!("END_{index}\n")),
                        "partial file in {style:?}"
                    );
                }
                assert!(result.omitted_files > 0);
            }
        }
        let markdown =
            generate_markdown(files.clone(), options, Arc::new(FileCache::new())).unwrap();
        assert!(counter.count_tokens(&markdown).unwrap() <= 1200);
    }
}

#[test]
fn tiny_budgets_return_a_clear_error_without_writing_partial_output() {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("source.rs"), "fn main() {}").unwrap();
    for style in ["markdown", "xml", "plain", "paths"] {
        let target = root.path().join(format!("{style}.out"));
        let mut command = assert_cmd::Command::cargo_bin("context-creator").unwrap();
        command
            .current_dir(root.path())
            .args(["--max-tokens", "0", "--style", style, "--output"])
            .arg(&target);
        if style == "paths" {
            command.assert().success();
            assert_eq!(fs::read_to_string(target).unwrap(), "");
        } else {
            command
                .assert()
                .failure()
                .stderr(predicates::str::contains("Token limit exceeded"));
            assert!(!target.exists());
        }
    }
}

#[test]
fn multiple_roots_share_one_budget_and_xml_document() {
    let project = TempDir::new().unwrap();
    let roots: Vec<_> = (0..3)
        .map(|index| project.path().join(format!("root{index}")))
        .collect();
    for root in &roots {
        fs::create_dir(root).unwrap();
        for index in 0..4 {
            fs::write(
                root.join(format!("source_{index}.rs")),
                "pub fn example() { println!(\"multi root\"); }\n".repeat(10),
            )
            .unwrap();
        }
    }
    let counter = TokenCounter::new().unwrap();
    for style in ["markdown", "xml", "plain", "paths"] {
        let output = assert_cmd::Command::cargo_bin("context-creator")
            .unwrap()
            .args(&roots)
            .args(["--max-tokens", "1000", "--style", style, "--quiet"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let output = String::from_utf8(output).unwrap();
        assert!(
            counter.count_tokens(&output).unwrap() <= 1000,
            "{style}: {output}"
        );
        if style == "xml" {
            assert_eq!(output.matches("<?xml").count(), 1);
            assert_eq!(output.matches("<context_creator>").count(), 1);
            assert!(output.ends_with("</context_creator>\n"));
        }
    }
}

#[test]
fn output_reservation_is_explicit_and_configurable() {
    let config = Config::parse_from([
        "context-creator",
        "--prompt",
        "hello",
        "--max-tokens",
        "5000",
        "--output-reserve",
        "2500",
    ]);
    let prompt = TokenCounter::new().unwrap().count_tokens("hello").unwrap();
    assert_eq!(config.get_effective_context_tokens(), Some(2500 - prompt));
    let zero = Config::parse_from([
        "context-creator",
        "--prompt",
        "hello",
        "--max-tokens",
        "5000",
        "--output-reserve",
        "0",
    ]);
    assert_eq!(zero.get_effective_context_tokens(), Some(5000 - prompt));
}
