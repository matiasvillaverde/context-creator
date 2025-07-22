//! Plain text formatter for context generation

use super::{DigestData, DigestFormatter};
use crate::core::walker::FileInfo;
use anyhow::Result;

/// Formatter that outputs plain text format
pub struct PlainFormatter {
    buffer: String,
}

impl PlainFormatter {
    /// Create a new PlainFormatter
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }
}

impl Default for PlainFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl DigestFormatter for PlainFormatter {
    fn render_header(&mut self, _data: &DigestData) -> Result<()> {
        self.buffer
            .push_str("================================================================\n");
        self.buffer.push_str("Code Digest\n");
        self.buffer
            .push_str("================================================================\n\n");
        Ok(())
    }

    fn render_statistics(&mut self, data: &DigestData) -> Result<()> {
        self.buffer.push_str("File Summary:\n");
        self.buffer
            .push_str(&format!("Total files: {}\n\n", data.files.len()));
        Ok(())
    }

    fn render_file_tree(&mut self, _data: &DigestData) -> Result<()> {
        // File tree not needed for basic plain format
        Ok(())
    }

    fn render_toc(&mut self, _data: &DigestData) -> Result<()> {
        // Table of contents not needed for basic plain format
        Ok(())
    }

    fn render_file_details(&mut self, file: &FileInfo, data: &DigestData) -> Result<()> {
        self.buffer
            .push_str("----------------------------------------------------------------\n");
        self.buffer
            .push_str(&format!("File: {}\n", file.relative_path.display()));
        self.buffer
            .push_str("----------------------------------------------------------------\n\n");

        // Read and add file content
        if let Ok(content) = data.cache.get_or_load(&file.path) {
            self.buffer.push_str(&content);
            self.buffer.push_str("\n\n");
        }
        Ok(())
    }

    fn finalize(self: Box<Self>) -> String {
        self.buffer
    }

    fn format_name(&self) -> &'static str {
        "plain"
    }
}
