//! Markdown formatter for context generation

use super::{DigestData, DigestFormatter};
use crate::core::context_builder::{
    format_import_names, format_imported_by_names, format_path_with_metadata, generate_file_tree,
    generate_statistics, get_language_hint, path_to_anchor,
};
use crate::core::walker::FileInfo;
use anyhow::Result;

/// Formatter that outputs standard Markdown format
pub struct MarkdownFormatter {
    buffer: String,
}

impl MarkdownFormatter {
    /// Create a new MarkdownFormatter
    pub fn new() -> Self {
        Self {
            buffer: String::with_capacity(1024 * 1024), // 1MB initial capacity
        }
    }
}

impl Default for MarkdownFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl DigestFormatter for MarkdownFormatter {
    fn render_header(&mut self, data: &DigestData) -> Result<()> {
        if !data.options.doc_header_template.is_empty() {
            let header = data
                .options
                .doc_header_template
                .replace("{directory}", data.base_directory);
            self.buffer.push_str(&header);
            self.buffer.push_str("\n\n");
        }
        Ok(())
    }

    fn render_statistics(&mut self, data: &DigestData) -> Result<()> {
        if data.options.include_stats {
            let stats = generate_statistics(data.files);
            self.buffer.push_str(&stats);
            self.buffer.push_str("\n\n");
        }
        Ok(())
    }

    fn render_file_tree(&mut self, data: &DigestData) -> Result<()> {
        if data.options.include_tree {
            let tree = generate_file_tree(data.files, data.options);
            self.buffer.push_str("## File Structure\n\n");
            self.buffer.push_str("```\n");
            self.buffer.push_str(&tree);
            self.buffer.push_str("```\n\n");
        }
        Ok(())
    }

    fn render_toc(&mut self, data: &DigestData) -> Result<()> {
        if data.options.include_toc {
            self.buffer.push_str("## Table of Contents\n\n");
            for file in data.files {
                let anchor = path_to_anchor(&file.relative_path);
                self.buffer.push_str(&format!(
                    "- [{path}](#{anchor})\n",
                    path = file.relative_path.display(),
                    anchor = anchor
                ));
            }
            self.buffer.push('\n');
        }
        Ok(())
    }

    fn render_file_details(&mut self, file: &FileInfo, data: &DigestData) -> Result<()> {
        // Add file header
        let path_with_metadata = format_path_with_metadata(file, data.options);
        let header = data
            .options
            .file_header_template
            .replace("{path}", &path_with_metadata);
        self.buffer.push_str(&header);
        self.buffer.push_str("\n\n");

        // Add semantic information
        add_markdown_semantic_info(&mut self.buffer, file);

        // Add file content
        if let Ok(content) = data.cache.get_or_load(&file.path) {
            let language = get_language_hint(&file.file_type);
            self.buffer.push_str(&format!("```{language}\n"));
            self.buffer.push_str(&content);
            if !content.ends_with('\n') {
                self.buffer.push('\n');
            }
            self.buffer.push_str("```\n\n");
        }

        Ok(())
    }

    fn finalize(self: Box<Self>) -> String {
        self.buffer
    }

    fn format_name(&self) -> &'static str {
        "markdown"
    }
}

fn add_markdown_semantic_info(output: &mut String, file: &FileInfo) {
    if !file.imports.is_empty() {
        output.push_str("Imports: ");
        let names = format_import_names(&file.imports);
        output.push_str(&format!("{}\n\n", names.join(", ")));
    }

    if !file.imported_by.is_empty() {
        output.push_str("Imported by: ");
        let names = format_imported_by_names(&file.imported_by);
        output.push_str(&format!("{}\n\n", names.join(", ")));
    }

    if !file.function_calls.is_empty() {
        output.push_str("Function calls: ");
        let names = format_function_call_names(&file.function_calls);
        output.push_str(&format!("{}\n\n", names.join(", ")));
    }

    if !file.type_references.is_empty() {
        output.push_str("Type references: ");
        let names = format_type_reference_names(&file.type_references);
        output.push_str(&format!("{}\n\n", names.join(", ")));
    }
}

fn format_function_call_names(
    calls: &[crate::core::semantic::analyzer::FunctionCall],
) -> Vec<String> {
    calls
        .iter()
        .map(|fc| {
            if let Some(module) = &fc.module {
                format!("{}.{}", module, fc.name)
            } else {
                fc.name.clone()
            }
        })
        .collect()
}

fn format_type_reference_names(
    refs: &[crate::core::semantic::analyzer::TypeReference],
) -> Vec<String> {
    refs.iter()
        .map(|tr| {
            if let Some(module) = &tr.module {
                if module.ends_with(&format!("::{}", tr.name)) {
                    module.clone()
                } else {
                    format!("{}.{}", module, tr.name)
                }
            } else {
                tr.name.clone()
            }
        })
        .collect()
}
