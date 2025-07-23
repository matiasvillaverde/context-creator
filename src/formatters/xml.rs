//! XML formatter for context generation

use super::{DigestData, DigestFormatter};
use crate::core::walker::FileInfo;
use anyhow::Result;

/// Formatter that outputs XML format
pub struct XmlFormatter {
    buffer: String,
    in_files_section: bool,
}

impl XmlFormatter {
    /// Create a new XmlFormatter
    pub fn new() -> Self {
        Self {
            buffer: String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<context_creator>\n"),
            in_files_section: false,
        }
    }
}

impl Default for XmlFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl DigestFormatter for XmlFormatter {
    fn render_header(&mut self, _data: &DigestData) -> Result<()> {
        // XML doesn't need a separate header
        Ok(())
    }

    fn render_statistics(&mut self, data: &DigestData) -> Result<()> {
        self.buffer.push_str("  <file_summary>\n");
        self.buffer.push_str(&format!(
            "    <total_files>{}</total_files>\n",
            data.files.len()
        ));
        self.buffer.push_str("  </file_summary>\n");
        Ok(())
    }

    fn render_file_tree(&mut self, _data: &DigestData) -> Result<()> {
        // File tree not needed for basic XML format
        Ok(())
    }

    fn render_toc(&mut self, _data: &DigestData) -> Result<()> {
        // Table of contents not needed for basic XML format
        Ok(())
    }

    fn render_file_details(&mut self, file: &FileInfo, data: &DigestData) -> Result<()> {
        // Start files section if not started
        if !self.in_files_section {
            self.buffer.push_str("  <files>\n");
            self.in_files_section = true;
        }

        // Read file content
        if let Ok(content) = data.cache.get_or_load(&file.path) {
            self.buffer.push_str(&format!(
                "    <file path=\"{}\">\n",
                file.relative_path.display()
            ));
            self.buffer.push_str("      <![CDATA[");
            self.buffer.push_str(&content);
            self.buffer.push_str("]]>\n");
            self.buffer.push_str("    </file>\n");
        }
        Ok(())
    }

    fn finalize(mut self: Box<Self>) -> String {
        // Close files section if it was opened
        if self.in_files_section {
            self.buffer.push_str("  </files>\n");
        }
        self.buffer.push_str("</context_creator>\n");
        self.buffer
    }

    fn format_name(&self) -> &'static str {
        "xml"
    }
}
