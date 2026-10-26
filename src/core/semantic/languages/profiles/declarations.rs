use super::{ancestors, child, text, Profile};
use crate::{
    core::semantic::{
        analyzer::SemanticResult,
        parsing::ParserExt,
        symbols::{CodeSymbol, SourceDeclaration},
    },
    utils::error::ContextCreatorError,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, OnceLock},
};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator, Tree};

impl Profile {
    fn tags(self) -> &'static str {
        match self {
            Self::Dart => tree_sitter_dart::TAGS_QUERY,
            Self::Lua => tree_sitter_lua::TAGS_QUERY,
            Self::Elixir => tree_sitter_elixir::TAGS_QUERY,
            Self::R => tree_sitter_r::TAGS_QUERY,
            Self::Bash => "(function_definition name: (_) @name) @definition.function\n(command name: (_) @name) @reference.call",
            Self::Scala => "(function_definition name: (_) @name) @definition.function\n(function_declaration name: (_) @name) @definition.function\n(class_definition name: (_) @name) @definition.type\n(object_definition name: (_) @name) @definition.type\n(trait_definition name: (_) @name) @definition.type\n(enum_definition name: (_) @name) @definition.type\n(type_definition name: (_) @name) @definition.type\n(call_expression function: (_) @name) @reference.call",
            Self::Julia => "(function_definition (signature (call_expression . (_) @name))) @definition.function\n(assignment . (call_expression . (_) @name)) @definition.function\n(struct_definition (type_head . (_) @name)) @definition.type\n(module_definition name: (_) @name) @definition.type\n(call_expression . (_) @name) @reference.call",
            Self::Haskell => "(function name: (_) @name) @definition.function\n(data_type name: (_) @name) @definition.type\n(newtype name: (_) @name) @definition.type\n(type_synonym name: (_) @name) @definition.type",
            Self::Clojure => "(list_lit . (sym_lit) @head . (sym_lit) @name (#any-of? @head \"defn\" \"defn-\" \"defmacro\" \"deftest\")) @definition.function\n(list_lit . (sym_lit) @head . (sym_lit) @name (#any-of? @head \"defrecord\" \"deftype\" \"defprotocol\")) @definition.type",
            Self::PowerShell => "(function_statement (function_name) @name) @definition.function\n(command command_name: (_) @name) @reference.call",
            Self::Perl => "(subroutine_declaration_statement name: (_) @name) @definition.function\n(function_call_expression function: (_) @name) @reference.call",
            Self::Sql => "(create_table (object_reference) @name) @definition.type\n(create_view (object_reference) @name) @definition.type\n(create_function (object_reference) @name) @definition.function",
            Self::Hcl => "(block (identifier) @name) @definition.type",
            Self::Css => "(rule_set (selectors) @name) @definition.style",
            Self::Html => "(element (start_tag (attribute (attribute_name) @key [(attribute_value) @name (quoted_attribute_value (attribute_value) @name)])) (#eq? @key \"id\")) @definition.element",
        }
    }
    pub(super) fn query(self) -> SemanticResult<Arc<Query>> {
        static QUERIES: OnceLock<Mutex<BTreeMap<Profile, Arc<Query>>>> = OnceLock::new();
        let mut queries = QUERIES
            .get_or_init(|| Mutex::new(BTreeMap::new()))
            .lock()
            .map_err(|_| {
                ContextCreatorError::ParseError("Language query cache unavailable".into())
            })?;
        if let Some(query) = queries.get(&self) {
            return Ok(query.clone());
        }
        let query = Arc::new(Query::new(&self.grammar(), self.tags()).map_err(|error| {
            ContextCreatorError::ParseError(format!("{} declaration query: {error}", self.name()))
        })?);
        queries.insert(self, query.clone());
        Ok(query)
    }
    pub(super) fn parse(self, source: &str) -> SemanticResult<Tree> {
        let mut parser = Parser::new();
        parser
            .set_language(&self.grammar())
            .map_err(|error| ContextCreatorError::ParseError(error.to_string()))?;
        parser.parse_bounded(source, None).ok_or_else(|| {
            ContextCreatorError::ParseError(format!("{} parsing timed out", self.name()))
        })
    }

    pub(crate) fn declarations(
        self,
        source: &str,
        tree: &Tree,
    ) -> SemanticResult<Vec<SourceDeclaration>> {
        let query = self.query()?;
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        let mut declarations = BTreeMap::new();
        while let Some(found) = matches.next() {
            let definition = found.captures.iter().find(|capture| {
                query.capture_names()[capture.index as usize].starts_with("definition.")
            });
            let name = found
                .captures
                .iter()
                .find(|capture| query.capture_names()[capture.index as usize] == "name");
            let (Some(definition), Some(name)) = (definition, name) else {
                continue;
            };
            if ancestors(definition.node).any(|node| node.kind() == "dis_expr") {
                continue;
            }
            let tag = query.capture_names()[definition.index as usize];
            let kind = if tag.ends_with("function") || tag.ends_with("method") {
                "function"
            } else if tag.ends_with("style") {
                "style"
            } else if tag.ends_with("element") {
                "element"
            } else if tag.ends_with("variable") || tag.ends_with("constant") {
                continue;
            } else {
                "type"
            };
            let mut envelope = definition.node;
            if self == Self::Dart
                && matches!(
                    envelope.kind(),
                    "method_signature" | "constructor_signature"
                )
            {
                if let Some(parent) = envelope
                    .parent()
                    .filter(|parent| parent.kind().ends_with("declaration"))
                {
                    envelope = parent;
                }
            }
            let name = if self == Self::Hcl {
                let mut children = envelope.walk();
                envelope
                    .named_children(&mut children)
                    .take_while(|child| child.kind() != "body")
                    .filter(|child| matches!(child.kind(), "identifier" | "string_lit"))
                    .map(|child| text(child, source).trim_matches('"'))
                    .collect::<Vec<_>>()
                    .join(".")
            } else if self == Self::Sql {
                text(name.node, source).to_owned()
            } else {
                text(name.node, source)
                    .trim_matches(['\'', '"'])
                    .to_string()
            };
            let body = envelope
                .child_by_field_name("body")
                .or_else(|| {
                    let mut children = envelope.walk();
                    let body = envelope.named_children(&mut children).find(|child| {
                        matches!(
                            child.kind(),
                            "block"
                                | "body"
                                | "function_body"
                                | "do_block"
                                | "script_block"
                                | "column_definitions"
                                | "indented_block"
                        )
                    });
                    body
                })
                .or_else(|| {
                    envelope
                        .child_by_field_name("rhs")
                        .and_then(|function| function.child_by_field_name("body"))
                });
            let end = if self == Self::Html {
                child(envelope, "start_tag").map(|tag| tag.end_byte())
            } else {
                None
            }
            .or_else(|| body.map(|body| body.start_byte()))
            .unwrap_or_else(|| {
                envelope.start_byte()
                    + text(envelope, source)
                        .find('\n')
                        .unwrap_or(envelope.end_byte() - envelope.start_byte())
            });
            let signature = source[envelope.start_byte()..end].trim().to_string();
            if signature.is_empty() || name.is_empty() {
                continue;
            }
            declarations.insert(
                (envelope.start_byte(), name.clone()),
                SourceDeclaration {
                    symbol: CodeSymbol {
                        name: name.clone(),
                        kind: kind.into(),
                        line: envelope.start_position().row + 1,
                        signature: signature.split_whitespace().collect::<Vec<_>>().join(" "),
                    },
                    qualified_name: name,
                    start_byte: envelope.start_byte(),
                    start_line: envelope.start_position().row + 1,
                    end_byte: envelope.end_byte(),
                    end_line: envelope.end_position().row
                        + usize::from(envelope.end_position().column > 0),
                    signature,
                },
            );
        }
        let mut declarations = declarations.into_values().collect::<Vec<_>>();
        let containers = declarations
            .iter()
            .map(|declaration| {
                (
                    declaration.start_byte,
                    declaration.end_byte,
                    declaration.symbol.name.clone(),
                )
            })
            .collect::<Vec<_>>();
        for declaration in &mut declarations {
            let mut names = containers
                .iter()
                .filter(|(start, end, _)| {
                    *start < declaration.start_byte && *end >= declaration.end_byte
                })
                .map(|(_, _, name)| name.clone())
                .collect::<Vec<_>>();
            names.push(declaration.symbol.name.clone());
            declaration.qualified_name = names.join("::");
        }
        Ok(declarations)
    }
}
