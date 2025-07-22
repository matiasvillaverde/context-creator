//! Paths-only formatter that outputs file paths without content

use super::{DigestData, DigestFormatter};
use crate::core::walker::FileInfo;
use anyhow::Result;

/// Formatter that outputs only file paths
pub struct PathsFormatter {
    buffer: String,
}

impl PathsFormatter {
    /// Create a new PathsFormatter
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }
}

impl Default for PathsFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl DigestFormatter for PathsFormatter {
    fn render_header(&mut self, _data: &DigestData) -> Result<()> {
        // Paths formatter doesn't render headers
        Ok(())
    }

    fn render_statistics(&mut self, _data: &DigestData) -> Result<()> {
        // Paths formatter doesn't render statistics
        Ok(())
    }

    fn render_file_tree(&mut self, _data: &DigestData) -> Result<()> {
        // Paths formatter doesn't render file tree
        Ok(())
    }

    fn render_toc(&mut self, _data: &DigestData) -> Result<()> {
        // Paths formatter doesn't render table of contents
        Ok(())
    }

    fn render_file_details(&mut self, file: &FileInfo, _data: &DigestData) -> Result<()> {
        // Only output the relative path
        self.buffer
            .push_str(&file.relative_path.display().to_string());
        self.buffer.push('\n');
        Ok(())
    }

    fn finalize(self: Box<Self>) -> String {
        self.buffer
    }

    fn format_name(&self) -> &'static str {
        "paths"
    }
}
