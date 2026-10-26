//! Shared AST extraction for languages whose declarations have similar shapes.
//! Resolution stays local; compiler classpaths, preprocessors and runtime loads
//! are deliberately not executed.

use super::super::{
    analyzer::{
        AnalysisResult, FunctionCall, FunctionDefinition, Import, LanguageAnalyzer,
        SemanticContext, SemanticResult, TypeReference,
    },
    parsing::ParserExt,
    path_validator::validate_import_path,
    resolver::{ModuleResolver, ResolvedPath},
    symbols::extract_symbols,
};
use crate::utils::error::ContextCreatorError;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    C,
    Cpp,
    CSharp,
    Kotlin,
    Ruby,
    Php,
}

impl Dialect {
    fn grammar(self) -> Language {
        match self {
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Self::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
            Self::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            Self::Php => tree_sitter_php::LANGUAGE_PHP.into(),
        }
    }
    fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::C => &["c", "h"],
            Self::Cpp => &["cpp", "cc", "cxx", "hpp", "hh", "hxx", "h"],
            Self::CSharp => &["cs"],
            Self::Kotlin => &["kt", "kts"],
            Self::Ruby => &["rb"],
            Self::Php => &["php"],
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::C => "C",
            Self::Cpp => "C++",
            Self::CSharp => "C#",
            Self::Kotlin => "Kotlin",
            Self::Ruby => "Ruby",
            Self::Php => "PHP",
        }
    }
    fn analyze(
        self,
        path: &Path,
        source: &str,
        context: &SemanticContext,
    ) -> SemanticResult<AnalysisResult> {
        let dialect = if self == Self::C && super::super::is_cpp_header(path, source) {
            Self::Cpp
        } else {
            self
        };
        let mut parser = Parser::new();
        parser
            .set_language(&dialect.grammar())
            .map_err(|error| ContextCreatorError::ParseError(error.to_string()))?;
        let tree = parser.parse_bounded(source, None).ok_or_else(|| {
            ContextCreatorError::ParseError(format!("{} parsing timed out", dialect.name()))
        })?;
        let mut result = AnalysisResult::default();
        if tree.root_node().has_error() {
            result
                .errors
                .push(format!("{} source contains syntax errors", dialect.name()));
        }
        let mut cursor = tree.walk();
        'walk: loop {
            let node = cursor.node();
            match node.kind() {
                "preproc_include" => {
                    if let Some(value) = node.child_by_field_name("path") {
                        let raw = text(value, source);
                        let module = quoted(raw).unwrap_or(raw);
                        result
                            .imports
                            .push(import(module, raw.starts_with('"'), node));
                    }
                }
                "using_directive" | "import" | "namespace_use_clause" => {
                    let mut children = node.walk();
                    let alias = node
                        .child_by_field_name("alias")
                        .or_else(|| {
                            if dialect == Self::CSharp {
                                node.child_by_field_name("name")
                            } else {
                                None
                            }
                        })
                        .or_else(|| {
                            if dialect == Self::Kotlin && text(node, source).contains(" as ") {
                                node.named_children(&mut children)
                                    .filter(|child| child.kind() == "identifier")
                                    .last()
                            } else {
                                None
                            }
                        });
                    let name = node.named_children(&mut children).find(|child| {
                        Some(*child) != alias
                            && matches!(
                                child.kind(),
                                "qualified_identifier"
                                    | "qualified_name"
                                    | "namespace_name"
                                    | "identifier"
                                    | "name"
                            )
                    });
                    if let Some(name) = name {
                        let mut item = import(text(name, source), false, node);
                        if let Some(alias) = alias {
                            item.items.push(text(alias, source).into());
                        }
                        if text(node, source).contains('*') {
                            item.items.push("*".into());
                        }
                        result.imports.push(item);
                    }
                }
                "include_expression"
                | "include_once_expression"
                | "require_expression"
                | "require_once_expression" => {
                    if let Some(value) = literal_argument(node, source) {
                        result.imports.push(import(value, true, node));
                    }
                }
                "function_definition"
                | "function_declarator"
                | "function_declaration"
                | "method_declaration"
                | "constructor_declaration"
                | "method"
                | "singleton_method" => {
                    let nested_declarator = node.kind() == "function_declarator"
                        && node.parent().is_some_and(|parent| {
                            matches!(parent.kind(), "function_definition" | "pointer_declarator")
                        });
                    if let Some(name) = declaration_name(node).filter(|_| !nested_declarator) {
                        let mut children = node.walk();
                        let header_end = node
                            .child_by_field_name("body")
                            .or_else(|| {
                                node.named_children(&mut children).find(|child| {
                                    matches!(
                                        child.kind(),
                                        "function_body" | "block" | "compound_statement"
                                    )
                                })
                            })
                            .map(|body| body.start_byte())
                            .unwrap_or(node.end_byte());
                        let header = &source[node.start_byte()..header_end];
                        let private = header
                            .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
                            .any(|word| {
                                word == "private"
                                    || (word == "static" && matches!(dialect, Self::C | Self::Cpp))
                            });
                        result.exported_functions.push(FunctionDefinition {
                            name: text(name, source).into(),
                            is_exported: !private,
                            line: node.start_position().row + 1,
                        });
                    }
                }
                "call" if dialect == Self::Ruby => {
                    if let Some(method) = node.child_by_field_name("method") {
                        let name = text(method, source);
                        if matches!(name, "require" | "require_relative") {
                            if let Some(value) = literal_argument(node, source) {
                                let module = if name == "require_relative" {
                                    format!("./{value}")
                                } else {
                                    value.to_string()
                                };
                                result.imports.push(import(
                                    &module,
                                    name == "require_relative",
                                    node,
                                ));
                            }
                        } else {
                            result.function_calls.push(FunctionCall {
                                name: name.into(),
                                module: node
                                    .child_by_field_name("receiver")
                                    .map(|receiver| text(receiver, source).into()),
                                line: node.start_position().row + 1,
                            });
                        }
                    }
                }
                "call_expression"
                | "invocation_expression"
                | "function_call_expression"
                | "member_call_expression"
                | "scoped_call_expression" => {
                    let mut children = node.walk();
                    let callee = node
                        .child_by_field_name("function")
                        .or_else(|| node.child_by_field_name("name"))
                        .or_else(|| node.named_children(&mut children).next());
                    if let Some(callee) = callee {
                        let raw = text(callee, source);
                        let raw = raw.split('<').next().unwrap_or(raw);
                        let name = raw
                            .rsplit(['.', ':', '\\', '>'])
                            .next()
                            .unwrap_or(raw)
                            .trim();
                        if !name.is_empty() && !name.contains(['(', ')', ' ']) {
                            result.function_calls.push(FunctionCall {
                                name: name.into(),
                                module: node
                                    .child_by_field_name("object")
                                    .or_else(|| node.child_by_field_name("scope"))
                                    .map(|module| text(module, source).into()),
                                line: node.start_position().row + 1,
                            });
                        }
                    }
                }
                _ => {}
            }
            if is_type_reference(node, dialect) {
                let raw = text(node, source);
                let raw = raw.split('<').next().unwrap_or(raw);
                let name = raw
                    .rsplit(['.', '\\', ':'])
                    .next()
                    .unwrap_or(raw)
                    .trim_end_matches('?');
                if !name.is_empty() && name.chars().all(|ch| ch.is_alphanumeric() || ch == '_') {
                    result.type_references.push(TypeReference {
                        name: name.into(),
                        module: None,
                        line: node.start_position().row + 1,
                        definition_path: None,
                        is_external: false,
                        external_package: None,
                    });
                }
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
        dialect.resolve_types(path, context, &mut result)?;
        Ok(result)
    }

    fn resolve_types(
        self,
        path: &Path,
        context: &SemanticContext,
        result: &mut AnalysisResult,
    ) -> SemanticResult<()> {
        let resolver = StructuralResolver(self);
        let mut resolved = HashMap::new();
        let mut definitions: BTreeMap<PathBuf, HashSet<String>> = BTreeMap::new();
        for item in &result.imports {
            if let Ok(target) = resolver.resolve_import(&item.module, path, &context.base_dir) {
                if !target.is_external {
                    if let Ok(content) = read_local_source(&context.base_dir, &target.path) {
                        if let Ok(symbols) = extract_symbols(&target.path, &content) {
                            definitions.entry(target.path.clone()).or_default().extend(
                                symbols
                                    .into_iter()
                                    .filter(|symbol| symbol.kind == "type")
                                    .map(|symbol| symbol.name),
                            );
                        }
                    }
                    resolved.insert(item.module.clone(), target);
                }
            }
        }
        let imports = result.imports.clone();
        for reference in &mut result.type_references {
            if let Some((definition, _)) = definitions
                .iter()
                .find(|(_, names)| names.contains(&reference.name))
            {
                reference.definition_path = Some(definition.clone());
                continue;
            }
            let mut candidates = Vec::new();
            for item in &imports {
                let module = item.module.trim_start_matches('\\').trim_end_matches(".*");
                if item.items.iter().any(|item| item == &reference.name)
                    || module.rsplit(['.', '\\', '/']).next() == Some(reference.name.as_str())
                {
                    candidates.push(module.to_string());
                } else if matches!(self, Self::CSharp | Self::Kotlin | Self::Php) {
                    candidates.push(format!(
                        "{}{}{}",
                        module,
                        if self == Self::Php { "\\" } else { "." },
                        reference.name
                    ));
                }
            }
            candidates.push(reference.name.clone());
            for candidate in candidates {
                let target = if let Some(cached) = resolved.get(&candidate) {
                    cached.clone()
                } else {
                    let target = resolver.resolve_import(&candidate, path, &context.base_dir)?;
                    resolved.insert(candidate.clone(), target.clone());
                    target
                };
                if !target.is_external {
                    reference.definition_path = Some(target.path);
                    reference.module = Some(candidate.clone());
                    if !result.imports.iter().any(|item| item.module == candidate) {
                        result
                            .imports
                            .push(import_at_line(&candidate, reference.line));
                    }
                    break;
                }
            }
        }
        Ok(())
    }
}

fn import(module: &str, is_relative: bool, node: Node<'_>) -> Import {
    Import {
        module: module.into(),
        items: Vec::new(),
        is_relative,
        line: node.start_position().row + 1,
    }
}
fn import_at_line(module: &str, line: usize) -> Import {
    Import {
        module: module.into(),
        items: Vec::new(),
        is_relative: false,
        line,
    }
}
fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    source.get(node.byte_range()).unwrap_or_default()
}
fn quoted(value: &str) -> Option<&str> {
    let value = value.trim();
    let quote = value.chars().next()?;
    if !matches!(quote, '\'' | '"')
        || !value.ends_with(quote)
        || value.len() < 2
        || (value.contains("#{") || value.contains("${"))
    {
        return None;
    }
    Some(&value[1..value.len() - 1])
}
fn literal_argument<'a>(node: Node<'_>, source: &'a str) -> Option<&'a str> {
    let arguments = node.child_by_field_name("arguments").unwrap_or(node);
    let mut argument = arguments.named_child(0)?;
    for _ in 0..4 {
        if matches!(argument.kind(), "argument" | "parenthesized_expression") {
            argument = argument.named_child(0)?;
        } else {
            break;
        }
    }
    if matches!(
        argument.kind(),
        "string" | "encapsed_string" | "string_literal"
    ) {
        quoted(text(argument, source))
    } else {
        None
    }
}
fn declaration_name(node: Node<'_>) -> Option<Node<'_>> {
    if let Some(name) = node.child_by_field_name("name") {
        return Some(name);
    }
    let mut declarator = node.child_by_field_name("declarator")?;
    for _ in 0..16 {
        if matches!(
            declarator.kind(),
            "identifier" | "field_identifier" | "type_identifier"
        ) {
            return Some(declarator);
        }
        declarator = declarator
            .child_by_field_name("declarator")
            .or_else(|| declarator.child_by_field_name("name"))?;
    }
    None
}
fn is_type_reference(node: Node<'_>, dialect: Dialect) -> bool {
    if matches!(node.kind(), "type_identifier" | "user_type")
        || (dialect == Dialect::Ruby && node.kind() == "constant")
    {
        return true;
    }
    if !matches!(node.kind(), "identifier" | "name" | "qualified_name") {
        return false;
    }
    node.parent().is_some_and(|parent| {
        ["type", "returns", "return_type"]
            .iter()
            .any(|field| parent.child_by_field_name(field) == Some(node))
            || matches!(
                parent.kind(),
                "type_argument_list" | "type_arguments" | "base_list" | "named_type"
            )
    })
}

pub struct StructuralResolver(pub Dialect);

impl ModuleResolver for StructuralResolver {
    fn resolve_import(
        &self,
        module: &str,
        from_file: &Path,
        base_dir: &Path,
    ) -> SemanticResult<ResolvedPath> {
        if module.len() > 512 || module.contains(['\0', '%']) {
            return Err(ContextCreatorError::SecurityError(
                "Invalid or oversized module name".into(),
            ));
        }
        let base = base_dir
            .canonicalize()
            .map_err(ContextCreatorError::IoError)?;
        let from = from_file
            .canonicalize()
            .map_err(ContextCreatorError::IoError)?;
        validate_import_path(&base, &from)?;
        let mut candidates = Vec::new();
        if matches!(self.0, Dialect::C | Dialect::Cpp | Dialect::Ruby)
            || module.contains('/')
            || module.ends_with(".php")
        {
            let clean = module.trim_matches(['<', '>']);
            let relative = Path::new(clean);
            if relative.is_absolute() {
                return Err(ContextCreatorError::SecurityError(
                    "Absolute imports are not allowed".into(),
                ));
            }
            for root in [
                from.parent().unwrap_or(&base).to_path_buf(),
                base.join("include"),
                base.join("src"),
                base.join("lib"),
                base.clone(),
            ] {
                candidates.push(root.join(relative));
                for extension in self.0.extensions() {
                    candidates.push(root.join(relative).with_extension(extension));
                }
            }
        } else {
            let separator = if self.0 == Dialect::Php { '\\' } else { '.' };
            let parts: Vec<_> = module.trim_start_matches('\\').split(separator).collect();
            if parts.iter().any(|part| {
                part.is_empty()
                    || !part
                        .chars()
                        .all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '$' || ch == '*')
            }) {
                return Err(ContextCreatorError::SecurityError(
                    "Invalid qualified module name".into(),
                ));
            }
            if self.0 == Dialect::Php {
                let composer = base.join("composer.json");
                if let Ok(content) = read_local_source(&base, &composer) {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(autoload) = config
                            .pointer("/autoload/psr-4")
                            .and_then(|value| value.as_object())
                        {
                            for (prefix, directories) in autoload {
                                if let Some(suffix) =
                                    module.trim_start_matches('\\').strip_prefix(prefix)
                                {
                                    let directories = directories
                                        .as_array()
                                        .cloned()
                                        .unwrap_or_else(|| vec![directories.clone()]);
                                    for directory in
                                        directories.iter().filter_map(|value| value.as_str())
                                    {
                                        candidates.push(
                                            base.join(directory)
                                                .join(suffix.replace('\\', "/"))
                                                .with_extension("php"),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let mut roots = vec![
                base.clone(),
                base.join("src"),
                base.join("src/main/kotlin"),
                base.join("src/test/kotlin"),
            ];
            roots.extend(
                from.parent()
                    .into_iter()
                    .flat_map(Path::ancestors)
                    .take_while(|parent| parent.starts_with(&base))
                    .map(Path::to_path_buf),
            );
            for root in roots {
                for start in 0..parts.len() {
                    let target = root.join(parts[start..].join("/"));
                    for extension in self.0.extensions() {
                        candidates.push(target.with_extension(extension));
                    }
                    if self.0 == Dialect::Kotlin {
                        if let Some(parent) = target.parent() {
                            if let Ok(entries) = std::fs::read_dir(parent) {
                                let mut files: Vec<_> = entries
                                    .flatten()
                                    .map(|entry| entry.path())
                                    .filter(|path| {
                                        path.extension().and_then(|e| e.to_str()) == Some("kt")
                                    })
                                    .collect();
                                files.sort();
                                for file in files {
                                    let file = validate_import_path(&base, &file)?;
                                    if let Ok(content) = read_local_source(&base, &file) {
                                        if extract_symbols(&file, &content).is_ok_and(|symbols| {
                                            symbols.iter().any(|symbol| {
                                                Some(symbol.name.as_str()) == parts.last().copied()
                                            })
                                        }) {
                                            candidates.push(file);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for candidate in candidates {
            if candidate.is_file() {
                return Ok(ResolvedPath {
                    path: validate_import_path(&base, &candidate)?,
                    is_external: false,
                    confidence: 0.9,
                });
            }
        }
        Ok(ResolvedPath {
            path: module.into(),
            is_external: true,
            confidence: 0.0,
        })
    }
    fn get_file_extensions(&self) -> Vec<&'static str> {
        self.0.extensions().to_vec()
    }
    fn is_external_module(&self, module: &str) -> bool {
        match self.0 {
            Dialect::C | Dialect::Cpp => module.starts_with('<'),
            Dialect::CSharp => module == "System" || module.starts_with("System."),
            Dialect::Kotlin => module.starts_with("java.") || module.starts_with("kotlin."),
            Dialect::Ruby => matches!(module, "json" | "set" | "pathname" | "fileutils"),
            Dialect::Php => false,
        }
    }
}

fn read_local_source(root: &Path, path: &Path) -> SemanticResult<String> {
    let path = validate_import_path(root, path)?;
    if std::fs::metadata(&path)
        .map_err(ContextCreatorError::IoError)?
        .len()
        > 1_048_576
    {
        return Err(ContextCreatorError::ParseError(
            "Imported source exceeds 1 MiB".into(),
        ));
    }
    std::fs::read_to_string(path).map_err(ContextCreatorError::IoError)
}

macro_rules! analyzer {
    ($name:ident, $dialect:ident) => {
        #[derive(Default)]
        pub struct $name;
        impl $name {
            pub fn new() -> Self {
                Self
            }
        }
        impl LanguageAnalyzer for $name {
            fn language_name(&self) -> &'static str {
                Dialect::$dialect.name()
            }
            fn analyze_file(
                &self,
                path: &Path,
                source: &str,
                context: &SemanticContext,
            ) -> SemanticResult<AnalysisResult> {
                Dialect::$dialect.analyze(path, source, context)
            }
            fn can_handle_extension(&self, extension: &str) -> bool {
                Dialect::$dialect.extensions().contains(&extension)
            }
            fn supported_extensions(&self) -> Vec<&'static str> {
                Dialect::$dialect.extensions().to_vec()
            }
        }
    };
}
analyzer!(CAnalyzer, C);
analyzer!(CppAnalyzer, Cpp);
analyzer!(CSharpAnalyzer, CSharp);
analyzer!(KotlinAnalyzer, Kotlin);
analyzer!(RubyAnalyzer, Ruby);
analyzer!(PhpAnalyzer, Php);
