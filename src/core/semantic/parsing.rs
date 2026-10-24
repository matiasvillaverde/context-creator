//! Cooperative parse deadlines supported by modern tree-sitter grammars.

use std::ops::ControlFlow;
use std::time::{Duration, Instant};
use tree_sitter::{ParseOptions, Parser, Tree};

pub const DEFAULT_PARSE_TIMEOUT: Duration = Duration::from_secs(30);

pub trait ParserExt {
    fn parse_bounded(&mut self, source: impl AsRef<[u8]>, old_tree: Option<&Tree>) -> Option<Tree>;
}

impl ParserExt for Parser {
    fn parse_bounded(&mut self, source: impl AsRef<[u8]>, old_tree: Option<&Tree>) -> Option<Tree> {
        parse_with_timeout(self, source.as_ref(), old_tree, DEFAULT_PARSE_TIMEOUT)
    }
}

pub fn parse_with_timeout(
    parser: &mut Parser,
    source: &[u8],
    old_tree: Option<&Tree>,
    timeout: Duration,
) -> Option<Tree> {
    let start = Instant::now();
    let mut progress = |_: &tree_sitter::ParseState| {
        if start.elapsed() >= timeout {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };
    let mut input = |offset: usize, _| source.get(offset..).unwrap_or_default();
    let tree = parser.parse_with_options(
        &mut input,
        old_tree,
        Some(ParseOptions::new().progress_callback(&mut progress)),
    );
    if tree.is_none() {
        parser.reset();
    }
    tree
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timed_out_parsers_can_be_reused_without_resuming_stale_input() {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let large = "pub fn calculate(value: usize) -> usize { value + 1 }\n".repeat(50_000);
        assert!(parse_with_timeout(
            &mut parser,
            large.as_bytes(),
            None,
            Duration::from_micros(1)
        )
        .is_none());
        let tree = parser.parse_bounded("pub fn fresh() {}", None).unwrap();
        assert!(!tree.root_node().has_error());
        assert!(tree.root_node().to_sexp().contains("function_item"));
    }
}
