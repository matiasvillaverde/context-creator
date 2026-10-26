//! Reproducible local retrieval experiment; no model, credentials, or network calls.
//! Run with: cargo run --release --locked --example agent_workflow
use anyhow::Result;
use context_creator::{
    agent::{AgentService, ContextRequest, ContextResponse},
    core::token::TokenCounter,
};
use serde_json::json;
use std::{collections::BTreeSet, fs, path::Path, time::Instant};

fn write(root: &Path, path: &str, text: &str) -> Result<()> {
    let file = root.join(path);
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(file, text)?;
    Ok(())
}
fn metrics(response: &ContextResponse, elapsed: f64, expected: &[&str]) -> serde_json::Value {
    let found = response
        .snippets
        .iter()
        .map(|snippet| snippet.path.as_str())
        .collect::<BTreeSet<_>>();
    json!({"wall_ms":elapsed,"response_tokens":response.token_count,"snippets":response.snippets.len(),
        "content_snippets":response.snippets.iter().filter(|snippet| snippet.content.is_some()).count(),
        "content_bytes":response.snippets.iter().filter_map(|snippet|snippet.content.as_ref()).map(String::len).sum::<usize>(),
        "expected_paths_found":expected.iter().filter(|path| found.contains(**path)).count(),"expected_paths_total":expected.len(),
        "index":response.index,"stale":response.stale,"reset":response.reset})
}
fn main() -> Result<()> {
    let root = tempfile::TempDir::new()?;
    write(
        root.path(),
        "pyproject.toml",
        "[project]\nname = 'session-service'\nversion = '0.1.0'\n",
    )?;
    write(root.path(), "src/__init__.py", "")?;
    write(
        root.path(),
        "src/models.py",
        "class Session:\n    expires_at: int\n",
    )?;
    let padding = "    # Preserve complete source while fitting a task budget.\n".repeat(60);
    let expiry = format!("from .models import Session\n\ndef session_expired(session: Session, now: int) -> bool:\n{padding}    return now >= session.expires_at\n");
    write(root.path(), "src/expiry.py", &expiry)?;
    write(root.path(), "src/handler.py", "from .expiry import session_expired\nfrom .models import Session\n\ndef authorize(session: Session, now: int) -> bool:\n    return not session_expired(session, now)\n")?;
    write(root.path(), "tests/test_expiry.py", "from src.expiry import session_expired\n\ndef test_expiration():\n    assert session_expired(type('Session', (), {'expires_at': 10})(), 11)\n")?;
    write(
        root.path(),
        "config/session.toml",
        "[session]\nexpiration_seconds = 3600\n",
    )?;
    for index in 0..120 {
        write(
            root.path(),
            &format!("payments/operation_{index}.py"),
            &format!("def charge_{index}(amount: int) -> int:\n{padding}    return amount * 100\n"),
        )?;
    }
    let expected = [
        "src/expiry.py",
        "src/models.py",
        "src/handler.py",
        "tests/test_expiry.py",
        "config/session.toml",
    ];
    let counter = TokenCounter::new()?;
    let service = AgentService::new();
    let mut request = ContextRequest::new(root.path(), "Fix session expiration");
    request.session_id = Some("experiment".into());
    request.max_tokens = 8000;
    let start = Instant::now();
    let first = service.build_context(request.clone())?;
    let cold_ms = start.elapsed().as_secs_f64() * 1000.0;
    request.expected_revision = Some(first.revision.clone());
    let start = Instant::now();
    let unchanged = service.build_context(request.clone())?;
    let warm_ms = start.elapsed().as_secs_f64() * 1000.0;
    write(
        root.path(),
        "src/expiry.py",
        &expiry.replace("now >= session.expires_at", "now > session.expires_at + 30"),
    )?;
    request.expected_revision = Some(unchanged.revision.clone());
    let start = Instant::now();
    let delta = service.build_context(request)?;
    let delta_ms = start.elapsed().as_secs_f64() * 1000.0;
    for response in [&first, &unchanged, &delta] {
        assert_eq!(
            counter.count_tokens(&serde_json::to_string(response)?)?,
            response.token_count
        );
        assert!(response.token_count <= 8000);
        assert!(expected.iter().all(|path| response
            .snippets
            .iter()
            .any(|snippet| snippet.path == *path)));
    }
    assert!(
        unchanged.index.analysis_reused
            && unchanged
                .snippets
                .iter()
                .all(|snippet| snippet.content.is_none())
    );
    assert!(delta.stale && delta.index.declarations_reparsed == 1);
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"schema_version":1,"version":env!("CARGO_PKG_VERSION"),"corpus":"session-expiration-v1","distractor_files":120,"tokenizer":"o200k-base","max_tokens":8000,
        "cold":metrics(&first,cold_ms,&expected),"unchanged":metrics(&unchanged,warm_ms,&expected),"edited":metrics(&delta,delta_ms,&expected),"measures":"local path retrieval and delta behavior; not model answer quality"})
        )?
    );
    Ok(())
}
