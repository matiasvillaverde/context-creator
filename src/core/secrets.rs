//! Opt-in, high-confidence content rules. This is not an entropy/DLP scanner.

use anyhow::{anyhow, Result};
use clap::ValueEnum;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    ValueEnum,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum SecretPolicy {
    #[default]
    Off,
    Redact,
    Error,
}

struct Rule {
    name: &'static str,
    pattern: Regex,
}
static RULES: OnceLock<std::result::Result<Vec<Rule>, String>> = OnceLock::new();

fn rules() -> Result<&'static [Rule]> {
    RULES.get_or_init(|| {
        [
            ("private-key", r"(?s)-----BEGIN (?:[A-Z]+ )?PRIVATE KEY-----.*?-----END (?:[A-Z]+ )?PRIVATE KEY-----"),
            ("anthropic-token", r"\bsk-ant-[A-Za-z0-9_-]{20,}\b"),
            ("openai-token", r"\bsk-(?:(?:proj|svcacct)-[A-Za-z0-9_-]{20,}|[A-Za-z0-9]{32,})\b"),
            ("github-token", r"\b(?:gh[pousr]_[A-Za-z0-9]{36,255}|github_pat_[A-Za-z0-9_]{60,255})\b"),
            ("aws-access-key", r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b"),
            ("google-api-key", r"\bAIza[A-Za-z0-9_-]{35}\b"),
        ].into_iter().map(|(name, pattern)| Regex::new(pattern)
            .map(|pattern| Rule { name, pattern }).map_err(|error| error.to_string()))
            .collect()
    }).as_ref().map(Vec::as_slice).map_err(|error| anyhow!("Secret rule initialization failed: {error}"))
}

/// Error diagnostics contain rule IDs and locations, never the matched values.
pub fn apply_policy(path: &Path, content: &str, policy: SecretPolicy) -> Result<String> {
    if policy == SecretPolicy::Off {
        return Ok(content.into());
    }
    let mut matches = Vec::new();
    for rule in rules()? {
        for found in rule.pattern.find_iter(content) {
            matches.push((found.start(), found.end(), rule.name));
        }
    }
    matches.sort_unstable();
    if policy == SecretPolicy::Error {
        if let Some((start, _, rule)) = matches.first() {
            let line = content[..*start]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            return Err(
                crate::utils::error::ContextCreatorError::SecurityError(format!(
                    "Secret detected ({rule}) in {} at line {line}",
                    path.display()
                ))
                .into(),
            );
        }
        return Ok(content.into());
    }
    let mut redacted = String::with_capacity(content.len());
    let mut end = 0;
    for (start, next, rule) in matches {
        if start < end {
            continue;
        }
        redacted.push_str(&content[end..start]);
        redacted.push_str(&format!("[REDACTED:{rule}]"));
        // Retain source line positions for maps and paginated file reads.
        for _ in content[start..next].bytes().filter(|byte| *byte == b'\n') {
            redacted.push('\n');
        }
        end = next;
    }
    redacted.push_str(&content[end..]);
    Ok(redacted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_credentials_are_redacted_and_error_messages_do_not_echo_values() {
        for secret in [
            format!("ghp_{}", "a".repeat(36)),
            format!("sk-proj-{}", "a".repeat(48)),
            format!("sk-ant-api03-{}", "a".repeat(60)),
            format!("AKIA{}", "A".repeat(16)),
            format!("AIza{}", "x".repeat(35)),
            [
                "-----BEGIN PRIVATE KEY-----",
                "fixture body",
                "-----END PRIVATE KEY-----",
            ]
            .join("\n"),
        ] {
            let source = format!("first\nkey = {secret}\nlast\n");
            let redacted =
                apply_policy(Path::new("source.txt"), &source, SecretPolicy::Redact).unwrap();
            assert!(!redacted.contains(&secret));
            assert!(redacted.contains("[REDACTED:"));
            assert_eq!(redacted.lines().count(), source.lines().count());
            let error = apply_policy(Path::new("source.txt"), &source, SecretPolicy::Error)
                .unwrap_err()
                .to_string();
            assert!(error.contains("line 2"));
            assert!(!error.contains(&secret));
            assert_eq!(
                apply_policy(Path::new("source.txt"), &source, SecretPolicy::Off).unwrap(),
                source
            );
        }
    }

    #[test]
    fn normal_source_and_documentation_placeholders_are_retained() {
        let source = "let password = \"example\"; // OPENAI_API_KEY=<YOUR_API_KEY>\n";
        assert_eq!(
            apply_policy(Path::new("test.rs"), source, SecretPolicy::Redact).unwrap(),
            source
        );
        assert!(apply_policy(Path::new("test.rs"), source, SecretPolicy::Error).is_ok());
    }
}
