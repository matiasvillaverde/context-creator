//! Opt-in provider accounting for the supplied user text.
//! Installed CLIs may add instructions and tools that this request cannot see.

use crate::cli::{Config, LlmTool};
use anyhow::{anyhow, bail, Result};
use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, AUTHORIZATION};
use serde::Serialize;
use serde_json::{json, Value};
use std::io::Read;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct NativeTokenCount {
    pub provider: &'static str,
    pub model: String,
    pub input_tokens: usize,
    pub scope: &'static str,
}

#[derive(Clone, Copy)]
enum Provider {
    OpenAi,
    Anthropic,
    Gemini,
}

impl Provider {
    fn for_tool(tool: LlmTool) -> Result<Self> {
        match tool {
            LlmTool::Codex => Ok(Self::OpenAi),
            LlmTool::Claude => Ok(Self::Anthropic),
            LlmTool::Gemini => Ok(Self::Gemini),
            LlmTool::Ollama => bail!("Native token counting supports Codex, Claude, and Gemini"),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
        }
    }
    fn endpoint(self, model: &str) -> String {
        match self {
            Self::OpenAi => "https://api.openai.com/v1/responses/input_tokens".into(),
            Self::Anthropic => "https://api.anthropic.com/v1/messages/count_tokens".into(),
            Self::Gemini => format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{model}:countTokens"
            ),
        }
    }
    fn credential(self) -> Result<String> {
        let variables: &[&str] = match self {
            Self::OpenAi => &["OPENAI_API_KEY"],
            Self::Anthropic => &["ANTHROPIC_API_KEY"],
            Self::Gemini => &["GOOGLE_API_KEY", "GEMINI_API_KEY"],
        };
        for variable in variables {
            if let Ok(value) = std::env::var(variable) {
                if !value.trim().is_empty() {
                    return Ok(value);
                }
            }
        }
        bail!("Native token counting requires {}", variables.join(" or "))
    }
    fn payload(self, model: &str, text: &str) -> Value {
        match self {
            Self::OpenAi => json!({"model": model, "input": text}),
            Self::Anthropic => {
                json!({"model": model, "messages": [{"role": "user", "content": text}]})
            }
            Self::Gemini => json!({"contents": [{"role": "user", "parts": [{"text": text}]}]}),
        }
    }
}

pub fn validate_config(config: &Config) -> Result<()> {
    if config.native_token_count {
        Provider::for_tool(config.llm_tool)?;
        validated_model(config)?;
    }
    Ok(())
}

fn validated_model(config: &Config) -> Result<&str> {
    let model = config.model.as_deref().ok_or_else(|| {
        anyhow!("--native-token-count requires --model with a full provider API model ID")
    })?;
    let model = model.strip_prefix("models/").unwrap_or(model);
    if model.is_empty()
        || model.len() > 200
        || !model
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || b"._-".contains(&ch))
    {
        bail!("Native counting requires a valid provider API model ID")
    }
    Ok(model)
}

/// No credentials are read and no client is built unless explicitly requested.
pub fn count_if_requested(config: &Config, context: &str) -> Result<Option<NativeTokenCount>> {
    if !config.native_token_count {
        return Ok(None);
    }
    let provider = Provider::for_tool(config.llm_tool)?;
    let model = validated_model(config)?;
    let text = config.get_prompt().map_or_else(
        || context.to_string(),
        |prompt| format!("{prompt}\n\n{context}"),
    );
    let credential = provider.credential()?;
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        .build()
        .map_err(|_| anyhow!("Failed to initialize native token-count client"))?;
    let input_tokens = request_count(
        &client,
        provider,
        &provider.endpoint(model),
        &provider.payload(model, &text),
        &credential,
    )?;
    if let Some(limit) = config.get_effective_max_tokens() {
        let count = input_tokens.saturating_add(config.get_output_reserve());
        if count > limit {
            return Err(
                crate::utils::error::ContextCreatorError::TokenLimitExceeded {
                    current: count,
                    max: limit,
                }
                .into(),
            );
        }
    }
    Ok(Some(NativeTokenCount {
        provider: provider.name(),
        model: model.into(),
        input_tokens,
        scope: "supplied-user-text; excludes CLI-added instructions and tools",
    }))
}

fn request_count(
    client: &Client,
    provider: Provider,
    endpoint: &str,
    payload: &Value,
    credential: &str,
) -> Result<usize> {
    let (header, value) = if matches!(provider, Provider::Gemini) {
        ("x-goog-api-key", credential.to_string())
    } else {
        (AUTHORIZATION.as_str(), format!("Bearer {credential}"))
    };
    let mut value = HeaderValue::from_str(&value)
        .map_err(|_| anyhow!("Invalid native token-count credential"))?;
    value.set_sensitive(true);
    let mut request = client.post(endpoint).header(header, value).json(payload);
    if matches!(provider, Provider::Anthropic) {
        request = request.header("anthropic-version", "2026-10-21");
    }
    // Do not include provider response bodies, URLs, or request data in errors.
    let response = request
        .send()
        .map_err(|_| anyhow!("Native token-count request failed"))?;
    if !response.status().is_success() {
        bail!(
            "Native token-count request returned HTTP {}",
            response.status().as_u16()
        );
    }
    let mut bytes = Vec::new();
    response
        .take(65_537)
        .read_to_end(&mut bytes)
        .map_err(|_| anyhow!("Failed to read native token-count response"))?;
    if bytes.len() > 65_536 {
        bail!("Native token-count response exceeded 64 KiB");
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow!("Invalid native token-count response"))?;
    let field = if matches!(provider, Provider::Gemini) {
        "totalTokens"
    } else {
        "input_tokens"
    };
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| anyhow!("Native token-count response has no valid count"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn mock(provider: Provider, response: &str, status: u16) -> Result<usize> {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/count", listener.local_addr().unwrap());
        let response = response.to_string();
        let server = std::thread::spawn(move || {
            use std::io::Write;
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buf = [0; 4096];
            loop {
                let read = stream.read(&mut buf).unwrap();
                assert_ne!(read, 0);
                bytes.extend_from_slice(&buf[..read]);
                if let Some(split) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..split]).to_lowercase();
                    let size: usize = headers
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length: ")
                                .and_then(|size| size.parse().ok())
                        })
                        .unwrap();
                    if bytes.len() >= split + 4 + size {
                        assert!(headers.starts_with("post /count "));
                        assert!(headers.contains(if matches!(provider, Provider::Gemini) {
                            "x-goog-api-key: test-credential"
                        } else {
                            "authorization: bearer test-credential"
                        }));
                        if matches!(provider, Provider::Anthropic) {
                            assert!(headers.contains("anthropic-version: 2026-10-21"));
                        }
                        let body: Value = serde_json::from_slice(&bytes[split + 4..]).unwrap();
                        assert_eq!(body, provider.payload("test-model", "你好 source"));
                        break;
                    }
                }
            }
            write!(stream, "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).unwrap();
        });
        let client = Client::builder()
            .timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let result = request_count(
            &client,
            provider,
            &endpoint,
            &provider.payload("test-model", "你好 source"),
            "test-credential",
        );
        server.join().unwrap();
        result
    }

    #[test]
    fn all_provider_contracts_are_tested_without_live_credentials() {
        for provider in [Provider::OpenAi, Provider::Anthropic, Provider::Gemini] {
            let response = if matches!(provider, Provider::Gemini) {
                r#"{"totalTokens":42}"#
            } else {
                r#"{"input_tokens":42}"#
            };
            assert_eq!(mock(provider, response, 200).unwrap(), 42);
        }
    }

    #[test]
    fn errors_never_echo_provider_bodies_or_credentials() {
        for (body, status) in [
            ("test-credential source", 401),
            ("not json", 200),
            (r#"{"input_tokens":-1}"#, 200),
            ("{}", 200),
            ("redirect", 307),
        ] {
            let error = mock(Provider::OpenAi, body, status)
                .unwrap_err()
                .to_string();
            assert!(!error.contains("test-credential"));
            assert!(!error.contains("source"));
            assert!(!error.contains("http://"));
        }
    }

    #[test]
    fn native_budget_adds_response_reservation_without_overflow() {
        let mut config = Config {
            native_token_count: true,
            model: Some("test-model".into()),
            llm_tool: LlmTool::Codex,
            max_tokens: Some(1500),
            output_reserve: Some(500),
            ..Config::default()
        };
        assert_eq!(config.get_effective_context_tokens(), Some(1000));
        config.output_reserve = Some(usize::MAX);
        assert_eq!(config.get_effective_context_tokens(), Some(0));
    }

    #[test]
    fn counting_is_off_by_default_and_model_paths_cannot_change_the_endpoint() {
        let mut config = Config::default();
        assert!(count_if_requested(&config, "source").unwrap().is_none());
        config.native_token_count = true;
        assert!(validate_config(&config).is_err());
        for invalid in ["../escape", "https://example.com", "model?key=source", ""] {
            config.model = Some(invalid.into());
            assert!(validate_config(&config).is_err());
        }
    }
}
