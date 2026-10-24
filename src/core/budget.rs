//! Enforce budgets on complete rendered documents, never on source substrings.

use crate::core::token::{TokenCounter, TokenEncoding};
use crate::utils::error::ContextCreatorError;
use anyhow::Result;

/// Account for separators and the complete input actually sent to a provider.
pub fn check_prompt_budget(config: &crate::cli::Config, context: &str) -> Result<()> {
    if let (Some(prompt), Some(limit)) = (config.get_prompt(), config.get_effective_max_tokens()) {
        let counter = TokenCounter::with_encoding(config.get_tokenizer())?;
        let count = counter
            .count_tokens(&format!("{prompt}\n\n{context}"))?
            .saturating_add(config.get_output_reserve());
        if count > limit {
            return Err(ContextCreatorError::TokenLimitExceeded {
                current: count,
                max: limit,
            }
            .into());
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct RenderedContext {
    pub text: String,
    pub file_count: usize,
    pub omitted_files: usize,
    /// Unbounded output is not tokenized unless the caller requests a report.
    pub token_count: Option<usize>,
}

/// Keep the highest-priority whole-file prefix whose actual rendering fits.
/// Candidate counts are always verified; no monotonicity assumption can let an
/// oversized document escape, even at BPE boundaries or changing metadata.
pub fn fit_rendered<T>(
    files: &[T],
    max_tokens: Option<usize>,
    encoding: TokenEncoding,
    mut render: impl FnMut(&[T]) -> Result<String>,
) -> Result<RenderedContext> {
    let text = render(files)?;
    let Some(limit) = max_tokens else {
        return Ok(RenderedContext {
            text,
            file_count: files.len(),
            omitted_files: 0,
            token_count: None,
        });
    };
    let counter = TokenCounter::with_encoding(encoding)?;
    let count = counter.count_tokens(&text)?;
    if count <= limit {
        return Ok(RenderedContext {
            text,
            file_count: files.len(),
            omitted_files: 0,
            token_count: Some(count),
        });
    }
    let empty = render(&[])?;
    let overhead = counter.count_tokens(&empty)?;
    if overhead > limit {
        return Err(ContextCreatorError::TokenLimitExceeded {
            current: overhead,
            max: limit,
        }
        .into());
    }
    let mut best = RenderedContext {
        text: empty,
        file_count: 0,
        omitted_files: files.len(),
        token_count: Some(overhead),
    };
    let (mut low, mut high) = (1, files.len());
    while low <= high {
        let middle = low + (high - low) / 2;
        let candidate = render(&files[..middle])?;
        let tokens = counter.count_tokens(&candidate)?;
        if tokens <= limit {
            best = RenderedContext {
                text: candidate,
                file_count: middle,
                omitted_files: files.len() - middle,
                token_count: Some(tokens),
            };
            low = middle + 1;
        } else {
            high = middle - 1;
        }
    }
    Ok(best)
}
