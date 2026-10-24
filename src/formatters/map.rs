//! A compact navigation map of declarations, without implementation bodies.

use super::{DigestData, DigestFormatter};
use crate::core::context_builder::format_path_for_output;
use crate::core::walker::FileInfo;
use anyhow::Result;

#[derive(Default)]
pub struct MapFormatter {
    buffer: String,
}

impl DigestFormatter for MapFormatter {
    fn render_header(&mut self, _data: &DigestData) -> Result<()> {
        self.buffer.push_str("# Repository map\n\n");
        Ok(())
    }
    fn render_statistics(&mut self, _data: &DigestData) -> Result<()> {
        Ok(())
    }
    fn render_file_tree(&mut self, _data: &DigestData) -> Result<()> {
        Ok(())
    }
    fn render_toc(&mut self, _data: &DigestData) -> Result<()> {
        Ok(())
    }
    fn render_file_details(&mut self, file: &FileInfo, data: &DigestData) -> Result<()> {
        self.buffer.push_str(&format!(
            "## {}\n",
            format_path_for_output(&file.relative_path)
        ));
        let symbols = data.cache.get_symbols(&file.path)?;
        for symbol in symbols.iter().take(64) {
            self.buffer
                .push_str(&format!("{}: {}\n", symbol.line, symbol.signature));
        }
        if symbols.len() > 64 {
            self.buffer
                .push_str("... more declarations; read this file for details\n");
        }
        self.buffer.push('\n');
        Ok(())
    }
    fn finalize(self: Box<Self>) -> String {
        self.buffer
    }
    fn format_name(&self) -> &'static str {
        "map"
    }
}
