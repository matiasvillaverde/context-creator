//! Language-specific project builders for acceptance tests

#![allow(dead_code)] // These builders will be used in later test phases
#![allow(clippy::uninlined_format_args)] // Keep traditional format! style

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Base trait for language-specific project builders
pub trait ProjectBuilder {
    fn build(self) -> (TempDir, PathBuf);
}

/// Python project builder for acceptance tests
pub struct PythonProjectBuilder {
    temp_dir: TempDir,
    files: Vec<(PathBuf, String)>,
}

impl PythonProjectBuilder {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().unwrap(),
            files: Vec::new(),
        }
    }

    pub fn add_file<P: AsRef<Path>>(mut self, path: P, content: &str) -> Self {
        self.files
            .push((path.as_ref().to_path_buf(), content.to_string()));
        self
    }

    /// Add a main.py file with imports
    pub fn with_main_imports(self, imports: &[&str]) -> Self {
        let import_statements = imports
            .iter()
            .map(|imp| format!("import {}", imp))
            .collect::<Vec<_>>()
            .join("\n");

        let content = format!(
            "{}\n\ndef main():\n    print('Python project')\n\nif __name__ == '__main__':\n    main()",
            import_statements
        );

        self.add_file("main.py", &content)
    }

    /// Add a module with functions
    pub fn with_module(self, name: &str, functions: &[&str]) -> Self {
        let func_defs = functions
            .iter()
            .map(|func| format!("def {}():\n    pass", func))
            .collect::<Vec<_>>()
            .join("\n\n");

        self.add_file(format!("{}.py", name), &func_defs)
    }
}

impl ProjectBuilder for PythonProjectBuilder {
    fn build(self) -> (TempDir, PathBuf) {
        let root = self.temp_dir.path().to_path_buf();

        for (path, content) in self.files {
            let full_path = root.join(&path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(full_path, content).unwrap();
        }

        (self.temp_dir, root)
    }
}

/// TypeScript project builder for acceptance tests
pub struct TypeScriptProjectBuilder {
    temp_dir: TempDir,
    files: Vec<(PathBuf, String)>,
}

impl TypeScriptProjectBuilder {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().unwrap(),
            files: Vec::new(),
        }
    }

    pub fn add_file<P: AsRef<Path>>(mut self, path: P, content: &str) -> Self {
        self.files
            .push((path.as_ref().to_path_buf(), content.to_string()));
        self
    }

    /// Add an index.ts file with imports
    pub fn with_index_imports(self, imports: &[(&str, &str)]) -> Self {
        let import_statements = imports
            .iter()
            .map(|(what, from)| format!("import {{ {} }} from '{}';", what, from))
            .collect::<Vec<_>>()
            .join("\n");

        let content = format!(
            "{}\n\nfunction main(): void {{\n    console.log('TypeScript project');\n}}\n\nmain();",
            import_statements
        );

        self.add_file("src/index.ts", &content)
    }

    /// Add a module with exported functions
    pub fn with_module(self, path: &str, exports: &[&str]) -> Self {
        let export_defs = exports
            .iter()
            .map(|func| {
                format!(
                    "export function {}(): void {{\n    // Implementation\n}}",
                    func
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        self.add_file(path, &export_defs)
    }

    /// Add an interface definition
    pub fn with_interface(
        self,
        path: &str,
        interface_name: &str,
        properties: &[(&str, &str)],
    ) -> Self {
        let props = properties
            .iter()
            .map(|(name, typ)| format!("    {}: {};", name, typ))
            .collect::<Vec<_>>()
            .join("\n");

        let content = format!("export interface {} {{\n{}\n}}", interface_name, props);

        self.add_file(path, &content)
    }
}

impl ProjectBuilder for TypeScriptProjectBuilder {
    fn build(self) -> (TempDir, PathBuf) {
        let root = self.temp_dir.path().to_path_buf();

        // Add package.json for TypeScript projects
        let package_json = r#"{
  "name": "test-project",
  "version": "1.0.0",
  "type": "module"
}"#;
        fs::write(root.join("package.json"), package_json).unwrap();

        for (path, content) in self.files {
            let full_path = root.join(&path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(full_path, content).unwrap();
        }

        (self.temp_dir, root)
    }
}

/// Rust project builder for acceptance tests
pub struct RustProjectBuilder {
    temp_dir: TempDir,
    files: Vec<(PathBuf, String)>,
}

impl RustProjectBuilder {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().unwrap(),
            files: Vec::new(),
        }
    }

    pub fn add_file<P: AsRef<Path>>(mut self, path: P, content: &str) -> Self {
        self.files
            .push((path.as_ref().to_path_buf(), content.to_string()));
        self
    }

    /// Add a main.rs file with use statements
    pub fn with_main_uses(self, uses: &[&str]) -> Self {
        let use_statements = uses
            .iter()
            .map(|u| format!("use {};", u))
            .collect::<Vec<_>>()
            .join("\n");

        let content = format!(
            "{}\n\nfn main() {{\n    println!(\"Rust project\");\n}}",
            use_statements
        );

        self.add_file("src/main.rs", &content)
    }

    /// Add a module with public functions
    pub fn with_module(self, name: &str, functions: &[&str]) -> Self {
        let func_defs = functions
            .iter()
            .map(|func| format!("pub fn {}() {{\n    // Implementation\n}}", func))
            .collect::<Vec<_>>()
            .join("\n\n");

        let content = format!("//! {} module\n\n{}", name, func_defs);

        self.add_file(format!("src/{}.rs", name), &content)
    }

    /// Add a struct definition
    pub fn with_struct(self, module: &str, struct_name: &str, fields: &[(&str, &str)]) -> Self {
        let field_defs = fields
            .iter()
            .map(|(name, typ)| format!("    pub {}: {},", name, typ))
            .collect::<Vec<_>>()
            .join("\n");

        let content = format!(
            "#[derive(Debug, Clone)]\npub struct {} {{\n{}\n}}",
            struct_name, field_defs
        );

        let existing = self
            .files
            .iter()
            .find(|(path, _)| path == &PathBuf::from(format!("src/{}.rs", module)))
            .map(|(_, content)| content.clone())
            .unwrap_or_default();

        let new_content = if existing.is_empty() {
            content
        } else {
            format!("{}\n\n{}", existing, content)
        };

        // Remove old entry if exists
        let new_builder = Self {
            temp_dir: self.temp_dir,
            files: self
                .files
                .into_iter()
                .filter(|(path, _)| path != &PathBuf::from(format!("src/{}.rs", module)))
                .collect(),
        };

        new_builder.add_file(format!("src/{}.rs", module), &new_content)
    }
}

impl ProjectBuilder for RustProjectBuilder {
    fn build(self) -> (TempDir, PathBuf) {
        let root = self.temp_dir.path().to_path_buf();

        // Add Cargo.toml for Rust projects
        let cargo_toml = r#"[package]
name = "my_lib"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;
        fs::write(root.join("Cargo.toml"), cargo_toml).unwrap();

        // Ensure src directory exists
        fs::create_dir_all(root.join("src")).unwrap();

        for (path, content) in self.files {
            let full_path = root.join(&path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(full_path, content).unwrap();
        }

        (self.temp_dir, root)
    }
}

// Convenient constructors
impl Default for PythonProjectBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TypeScriptProjectBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RustProjectBuilder {
    fn default() -> Self {
        Self::new()
    }
}
