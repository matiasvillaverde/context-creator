//! Shared tree-sitter plumbing for application, scripting, data and markup languages.
//! Language-specific syntax and resolution remain explicit; nothing is executed.

mod declarations;
mod imports;
mod resolver;

use super::super::{
    analyzer::{
        AnalysisResult, FunctionCall, FunctionDefinition, LanguageAnalyzer, SemanticContext,
        SemanticResult, TypeReference,
    },
    query_engine::QueryEngine,
};
use crate::utils::error::ContextCreatorError;
use imports::{add_import, html_attributes};
pub use resolver::ProfileResolver;
use std::path::Path;
use tree_sitter::{Language, Node, Parser, QueryCursor, StreamingIterator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Profile {
    Bash,
    Dart,
    Lua,
    Scala,
    Elixir,
    R,
    Julia,
    Haskell,
    Clojure,
    PowerShell,
    Perl,
    Sql,
    Hcl,
    Html,
    Css,
}

pub(crate) fn for_file(path: &Path) -> Option<Profile> {
    Some(
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "sh" | "bash" => Profile::Bash,
            "dart" => Profile::Dart,
            "lua" => Profile::Lua,
            "scala" | "sc" => Profile::Scala,
            "ex" | "exs" => Profile::Elixir,
            "r" => Profile::R,
            "jl" => Profile::Julia,
            "hs" => Profile::Haskell,
            "clj" | "cljs" | "cljc" => Profile::Clojure,
            "ps1" | "psm1" | "psd1" => Profile::PowerShell,
            "pl" | "pm" => Profile::Perl,
            "sql" => Profile::Sql,
            "tf" | "tfvars" | "hcl" => Profile::Hcl,
            "html" | "htm" => Profile::Html,
            "css" => Profile::Css,
            _ => return None,
        },
    )
}

impl Profile {
    pub(crate) fn grammar(self) -> Language {
        match self {
            Self::Bash => tree_sitter_bash::LANGUAGE.into(),
            Self::Dart => tree_sitter_dart::LANGUAGE.into(),
            Self::Lua => tree_sitter_lua::LANGUAGE.into(),
            Self::Scala => tree_sitter_scala::LANGUAGE.into(),
            Self::Elixir => tree_sitter_elixir::LANGUAGE.into(),
            Self::R => tree_sitter_r::LANGUAGE.into(),
            Self::Julia => tree_sitter_julia::LANGUAGE.into(),
            Self::Haskell => tree_sitter_haskell::LANGUAGE.into(),
            Self::Clojure => tree_sitter_clojure_orchard::LANGUAGE.into(),
            Self::PowerShell => tree_sitter_powershell::LANGUAGE.into(),
            Self::Perl => tree_sitter_perl::LANGUAGE.into(),
            Self::Sql => tree_sitter_sequel::LANGUAGE.into(),
            Self::Hcl => tree_sitter_hcl::LANGUAGE.into(),
            Self::Html => tree_sitter_html::LANGUAGE.into(),
            Self::Css => tree_sitter_css::LANGUAGE.into(),
        }
    }
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Bash => "Bash",
            Self::Dart => "Dart",
            Self::Lua => "Lua",
            Self::Scala => "Scala",
            Self::Elixir => "Elixir",
            Self::R => "R",
            Self::Julia => "Julia",
            Self::Haskell => "Haskell",
            Self::Clojure => "Clojure",
            Self::PowerShell => "PowerShell",
            Self::Perl => "Perl",
            Self::Sql => "SQL",
            Self::Hcl => "HCL",
            Self::Html => "HTML",
            Self::Css => "CSS",
        }
    }
    fn extensions(self) -> Vec<&'static str> {
        match self {
            Self::Bash => vec!["sh", "bash"],
            Self::Dart => vec!["dart"],
            Self::Lua => vec!["lua"],
            Self::Scala => vec!["scala", "sc"],
            Self::Elixir => vec!["ex", "exs"],
            Self::R => vec!["r", "R"],
            Self::Julia => vec!["jl"],
            Self::Haskell => vec!["hs"],
            Self::Clojure => vec!["clj", "cljs", "cljc"],
            Self::PowerShell => vec!["ps1", "psm1", "psd1"],
            Self::Perl => vec!["pl", "pm"],
            Self::Sql => vec!["sql"],
            Self::Hcl => vec!["tf", "hcl", "tfvars"],
            Self::Html => vec!["html", "htm"],
            Self::Css => vec!["css"],
        }
    }
}

impl LanguageAnalyzer for Profile {
    fn language_name(&self) -> &'static str {
        self.name()
    }
    fn can_handle_extension(&self, extension: &str) -> bool {
        self.extensions()
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(extension))
    }
    fn supported_extensions(&self) -> Vec<&'static str> {
        self.extensions()
    }
    fn analyze_file(
        &self,
        path: &Path,
        source: &str,
        context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        let tree = self.parse(source)?;
        let mut result = AnalysisResult::default();
        if tree.root_node().has_error() {
            result
                .errors
                .push(format!("{} source contains syntax errors", self.name()));
        }
        let declarations = self.declarations(source, &tree)?;
        for declaration in &declarations {
            if declaration.symbol.kind == "function" {
                result.exported_functions.push(FunctionDefinition {
                    name: declaration.symbol.name.clone(),
                    is_exported: true,
                    line: declaration.start_line,
                });
            }
        }
        let query = self.query()?;
        let mut query_cursor = QueryCursor::new();
        let mut matches = query_cursor.matches(&query, tree.root_node(), source.as_bytes());
        while let Some(found) = matches.next() {
            let name = found
                .captures
                .iter()
                .find(|capture| query.capture_names()[capture.index as usize] == "name");
            let Some(name_capture) = name else {
                continue;
            };
            let name_text = text(name_capture.node, source);
            if found
                .captures
                .iter()
                .any(|capture| query.capture_names()[capture.index as usize] == "reference.call")
            {
                let (module, name) = name_text
                    .rsplit_once(['.', ':', '/'])
                    .map(|(module, name)| (Some(module.to_string()), name))
                    .unwrap_or((None, name_text));
                if !name.is_empty() && !name.contains(['(', ')', '\n']) {
                    result.function_calls.push(FunctionCall {
                        name: name.into(),
                        module,
                        line: name_capture.node.start_position().row + 1,
                    });
                }
            }
        }
        let mut cursor = tree.walk();
        'walk: loop {
            let node = cursor.node();
            self.imports(node, source, &mut result.imports);
            if *self == Self::Html
                && node.kind() == "raw_text"
                && node
                    .parent()
                    .is_some_and(|parent| parent.kind() == "script_element")
            {
                if let Some(parent) = node.parent() {
                    let attrs = child(parent, "start_tag")
                        .map(|tag| html_attributes(tag, source))
                        .unwrap_or_default();
                    let script_type = attrs.get("type").map(String::as_str).unwrap_or("");
                    if !attrs.contains_key("src")
                        && matches!(
                            script_type,
                            "" | "module" | "text/javascript" | "application/javascript"
                        )
                    {
                        let language = tree_sitter_javascript::LANGUAGE.into();
                        let mut parser = Parser::new();
                        parser
                            .set_language(&language)
                            .map_err(|error| ContextCreatorError::ParseError(error.to_string()))?;
                        let engine = QueryEngine::new(language, "javascript")?;
                        match engine.analyze_with_parser(&mut parser, text(node, source)) {
                            Ok(analysis) => result.imports.extend(
                                analysis.imports.into_iter().map(|mut import| {
                                    import.line += node.start_position().row;
                                    import
                                }),
                            ),
                            Err(_) => add_import(&mut result.imports, None, false, node),
                        }
                    }
                }
            }
            if matches!(node.kind(), "type_identifier" | "type_name") {
                let name = text(node, source);
                let definition = declarations.iter().find(|declaration| {
                    declaration.symbol.kind == "type" && declaration.symbol.name == name
                });
                result.type_references.push(TypeReference {
                    name: name.into(),
                    module: None,
                    line: node.start_position().row + 1,
                    definition_path: definition.map(|_| path.to_path_buf()),
                    is_external: definition.is_none() && builtin_type(name),
                    external_package: (definition.is_none() && builtin_type(name))
                        .then(|| self.name().to_owned()),
                });
            }
            if cursor.goto_first_child() {
                continue;
            }
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    break 'walk;
                }
            }
        }
        let _ = context;
        Ok(result)
    }
}

fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}
fn ancestors(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    std::iter::successors(Some(node), |node| node.parent())
}
fn child<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == kind);
    found
}
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn builtin_type(name: &str) -> bool {
    matches!(
        name,
        "void"
            | "int"
            | "bool"
            | "double"
            | "num"
            | "String"
            | "string"
            | "Object"
            | "dynamic"
            | "Any"
            | "AnyRef"
            | "Int"
            | "Integer"
            | "Bool"
            | "Boolean"
            | "Float"
            | "Double"
            | "Unit"
            | "Nothing"
            | "IO"
            | "Char"
            | "Word"
            | "List"
            | "Maybe"
            | "Either"
    )
}

macro_rules! facade {
    ($name:ident,$profile:ident) => {
        #[derive(Default)]
        pub struct $name;
        impl $name {
            pub fn new() -> Self {
                Self
            }
        }
        impl LanguageAnalyzer for $name {
            fn language_name(&self) -> &'static str {
                Profile::$profile.name()
            }
            fn can_handle_extension(&self, extension: &str) -> bool {
                Profile::$profile.can_handle_extension(extension)
            }
            fn supported_extensions(&self) -> Vec<&'static str> {
                Profile::$profile.extensions()
            }
            fn analyze_file(
                &self,
                path: &Path,
                source: &str,
                context: &SemanticContext,
            ) -> SemanticResult<AnalysisResult> {
                Profile::$profile.analyze_file(path, source, context)
            }
        }
    };
}
facade!(DartAnalyzer, Dart);
facade!(LuaAnalyzer, Lua);
facade!(ScalaAnalyzer, Scala);
facade!(ElixirAnalyzer, Elixir);
facade!(RAnalyzer, R);
facade!(JuliaAnalyzer, Julia);
