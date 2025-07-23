//! Output formatters for different file formats

use crate::cli::OutputFormat;
use crate::core::cache::FileCache;
use crate::core::context_builder::ContextOptions;
use crate::core::walker::FileInfo;
use anyhow::Result;
use std::sync::Arc;

pub mod markdown;
pub mod paths;
pub mod plain;
pub mod xml;

/// Data passed to formatters for rendering
pub struct DigestData<'a> {
    pub files: &'a [FileInfo],
    pub options: &'a ContextOptions,
    pub cache: &'a Arc<FileCache>,
    pub base_directory: &'a str,
}

/// Trait for digest formatters
pub trait DigestFormatter {
    /// Render the document header
    fn render_header(&mut self, data: &DigestData) -> Result<()>;

    /// Render statistics section
    fn render_statistics(&mut self, data: &DigestData) -> Result<()>;

    /// Render file tree structure
    fn render_file_tree(&mut self, data: &DigestData) -> Result<()>;

    /// Render table of contents
    fn render_toc(&mut self, data: &DigestData) -> Result<()>;

    /// Render details for a single file
    fn render_file_details(&mut self, file: &FileInfo, data: &DigestData) -> Result<()>;

    /// Finalize and return the formatted output
    fn finalize(self: Box<Self>) -> String;

    /// Get the format name (for testing)
    fn format_name(&self) -> &'static str;
}

/// Create a formatter based on the output format
pub fn create_formatter(format: OutputFormat) -> Box<dyn DigestFormatter> {
    match format {
        OutputFormat::Markdown => Box::new(markdown::MarkdownFormatter::new()),
        OutputFormat::Xml => Box::new(xml::XmlFormatter::new()),
        OutputFormat::Plain => Box::new(plain::PlainFormatter::new()),
        OutputFormat::Paths => Box::new(paths::PathsFormatter::new()),
    }
}
