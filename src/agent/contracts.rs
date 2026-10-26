//! Evidence-based HTTP and schema hints. These are static inferences, never runtime dispatch.
use super::{index::Snapshot, types::*};
use crate::core::semantic::parsing::ParserExt;
use anyhow::Result;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
struct Endpoint {
    location: Location,
    method: String,
    route: String,
    role: &'static str,
    schemas: BTreeSet<String>,
}

pub(crate) fn relationships(snapshot: &Snapshot) -> Result<(Vec<Relation>, Vec<Gap>)> {
    let method_call = Regex::new(r"(?i)(?:^|\.)(get|post|put|patch|delete|head|options)$")?;
    let literal = Regex::new(r#"[\"'`](/[^\"'`\s]*)[\"'`]"#)?;
    let parameter = Regex::new(r"\{[^/{}]+\}|:[A-Za-z_][A-Za-z_0-9]*")?;
    let mut endpoints = Vec::new();
    let mut gaps = Vec::new();
    for (path, source) in &snapshot.sources {
        if endpoints.len() >= 4096 {
            break;
        }
        if let Some(language) =
            crate::core::semantic::grammar_for_source(&source.info.path, &source.text)
        {
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(&language)?;
            if let Some(tree) = parser.parse_bounded(source.text.as_ref(), None) {
                let mut cursor = tree.walk();
                'walk: loop {
                    let node = cursor.node();
                    if matches!(
                        node.kind(),
                        "call_expression"
                            | "call"
                            | "method_invocation"
                            | "attribute_item"
                            | "annotation"
                    ) {
                        let text = &source.text[node.byte_range()];
                        let function = node
                            .child_by_field_name("function")
                            .or_else(|| node.child_by_field_name("name"))
                            .map(|part| &source.text[part.byte_range()])
                            .unwrap_or_else(|| text.split('(').next().unwrap_or(""));
                        let lower = function.trim().to_ascii_lowercase();
                        let method = method_call
                            .captures(&lower)
                            .and_then(|captures| captures.get(1))
                            .map(|value| value.as_str().to_uppercase());
                        let receiver = lower.split('.').next().unwrap_or("");
                        let recognized = [
                            "app", "router", "route", "api", "client", "http", "axios", "requests",
                            "session",
                        ]
                        .contains(&receiver);
                        let is_fetch = lower == "fetch" || lower.ends_with(".fetch");
                        let attribute =
                            node.kind() == "attribute_item" || node.kind() == "annotation";
                        let attribute_method = if attribute {
                            ["get", "post", "put", "patch", "delete", "head", "options"]
                                .into_iter()
                                .find(|method| {
                                    lower
                                        .trim_start_matches("#[")
                                        .trim_start_matches('@')
                                        .starts_with(&format!("{method}("))
                                        || lower.contains(&format!("{method}mapping"))
                                        || lower.trim_start_matches("#[").trim_start_matches('@')
                                            == *method
                                })
                                .map(str::to_uppercase)
                        } else {
                            None
                        };
                        let route_builder =
                            lower.ends_with(".route") && ["app", "router"].contains(&receiver);
                        let route_method = if route_builder {
                            ["get", "post", "put", "patch", "delete"]
                                .into_iter()
                                .find(|method| text.contains(&format!(", {method}(")))
                                .map(str::to_uppercase)
                        } else {
                            None
                        };
                        let method = if is_fetch {
                            fetch_method(node, &source.text)
                        } else {
                            attribute_method
                                .or(route_method)
                                .or(method.filter(|_| recognized))
                        };
                        if is_fetch && method.is_none() {
                            gaps.push(Gap {
                                kind: "dynamic_contract".into(),
                                path: Some(path.clone()),
                                line: Some(node.start_position().row + 1),
                                detail:
                                    "Computed fetch options prevent static HTTP method resolution"
                                        .into(),
                            });
                        }
                        if let Some(method) = method {
                            let route_literal = first_literal(node, &source.text, attribute);
                            if let Some(route) = route_literal
                                .and_then(|text| literal.captures(text))
                                .and_then(|captures| captures.get(1))
                            {
                                let route = route.as_str().split('?').next().unwrap_or("");
                                if !route.contains("${")
                                    && !route.contains('\\')
                                    && route.len() <= 512
                                {
                                    let route = parameter
                                        .replace_all(route, "{}")
                                        .trim_end_matches('/')
                                        .to_string();
                                    let role = if attribute
                                        || route_builder
                                        || ["app", "router", "route", "api"].contains(&receiver)
                                    {
                                        "server"
                                    } else {
                                        "client"
                                    };
                                    endpoints.push(Endpoint {
                                        location: Location {
                                            path: path.clone(),
                                            line: node.start_position().row + 1,
                                        },
                                        method,
                                        route,
                                        role,
                                        schemas: BTreeSet::new(),
                                    });
                                } else {
                                    gaps.push(Gap { kind: "dynamic_contract".into(), path: Some(path.clone()), line: Some(node.start_position().row + 1), detail: "Computed HTTP path needs explicit contract or seed evidence".into() });
                                }
                            } else {
                                gaps.push(Gap {
                                    kind: "dynamic_contract".into(),
                                    path: Some(path.clone()),
                                    line: Some(node.start_position().row + 1),
                                    detail: "HTTP call has no supported literal relative path"
                                        .into(),
                                });
                            }
                        }
                    }
                    if endpoints.len() >= 4096 {
                        break 'walk;
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
            }
        }
        if source.info.path.extension().and_then(|part| part.to_str()) == Some("json") {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&source.text) {
                if value.get("openapi").is_some() || value.get("swagger").is_some() {
                    if let Some(paths) = value.get("paths").and_then(|paths| paths.as_object()) {
                        for (route, methods) in paths {
                            if let Some(methods) = methods.as_object() {
                                for (method, operation) in methods {
                                    if !["get", "post", "put", "patch", "delete", "head", "options"]
                                        .contains(&method.as_str())
                                    {
                                        continue;
                                    }
                                    let mut refs = BTreeSet::new();
                                    collect_refs(operation, &mut refs);
                                    let line = source
                                        .text
                                        .lines()
                                        .position(|line| line.contains(&format!("\"{route}\"")))
                                        .unwrap_or(0)
                                        + 1;
                                    endpoints.push(Endpoint {
                                        location: Location {
                                            path: path.clone(),
                                            line,
                                        },
                                        method: method.to_uppercase(),
                                        route: parameter
                                            .replace_all(route.trim_end_matches('/'), "{}")
                                            .into(),
                                        role: "contract",
                                        schemas: refs,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let mut groups: BTreeMap<(&str, &str), Vec<&Endpoint>> = BTreeMap::new();
    for endpoint in &endpoints {
        groups
            .entry((&endpoint.method, &endpoint.route))
            .or_default()
            .push(endpoint);
    }
    let mut relations = Vec::new();
    let mut types = BTreeMap::<&str, Vec<Location>>::new();
    for (path, source) in &snapshot.sources {
        for declaration in source
            .declarations
            .iter()
            .filter(|declaration| declaration.symbol.kind == "type")
        {
            types
                .entry(&declaration.symbol.name)
                .or_default()
                .push(Location {
                    path: path.clone(),
                    line: declaration.symbol.line,
                });
        }
    }
    for ((method, route), group) in groups {
        let servers = group
            .iter()
            .filter(|endpoint| endpoint.role == "server")
            .count();
        if servers > 1 {
            for endpoint in &group {
                gaps.push(Gap { kind: "ambiguous_contract".into(), path: Some(endpoint.location.path.clone()), line: Some(endpoint.location.line), detail: format!("{servers} possible handlers for {method} {route}; service origin is unknown") });
            }
        }
        for (index, left) in group.iter().enumerate() {
            for right in group.iter().skip(index + 1) {
                if left.location.path != right.location.path
                    && left.role != right.role
                    && relations.len() < 1024
                {
                    relations.push(Relation { from: left.location.clone(), to: right.location.clone(), kind: "http_contract".into(), confidence: Confidence::Inferred, evidence: format!("{} and {} share literal {method} {route}; origin and runtime routing unverified", left.role, right.role) });
                }
            }
        }
        for endpoint in &group {
            for schema in &endpoint.schemas {
                if let Some(locations) = types.get(schema.as_str()) {
                    for location in locations {
                        if relations.len() < 1024 {
                            relations.push(Relation { from: endpoint.location.clone(), to: location.clone(), kind: "schema_type".into(), confidence: Confidence::Inferred, evidence: format!("OpenAPI $ref {schema} matches a source type name; generated mapping unverified") });
                        }
                    }
                }
            }
        }
    }
    if endpoints.len() >= 4096 || relations.len() >= 1024 {
        gaps.push(Gap {
            kind: "contract_limit".into(),
            path: None,
            line: None,
            detail: "Contract graph is bounded to 4096 endpoints and 1024 relationships".into(),
        });
    }
    Ok((relations, gaps))
}

fn arguments(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    if node.kind() == "attribute_item" {
        let mut cursor = node.walk();
        let attribute = node
            .named_children(&mut cursor)
            .find(|child| child.kind() == "attribute")?;
        return arguments(attribute);
    }
    node.child_by_field_name("arguments").or_else(|| {
        let mut cursor = node.walk();
        let result = node.named_children(&mut cursor).find(|child| {
            matches!(
                child.kind(),
                "arguments" | "argument_list" | "annotation_argument_list" | "token_tree"
            )
        });
        result
    })
}

fn first_literal<'a>(
    node: tree_sitter::Node<'_>,
    source: &'a str,
    attribute: bool,
) -> Option<&'a str> {
    let root = arguments(node)?;
    let mut cursor = root.walk();
    let first = root
        .named_children(&mut cursor)
        .find(|child| !child.kind().contains("comment"))?;
    let literal = if attribute {
        let mut nodes = vec![first];
        let mut found = None;
        while let Some(node) = nodes.pop() {
            if node.kind().contains("comment") {
                continue;
            }
            if matches!(
                node.kind(),
                "string" | "string_literal" | "interpreted_string_literal"
            ) {
                found = Some(node);
                break;
            }
            let mut walk = node.walk();
            nodes.extend(node.named_children(&mut walk));
        }
        found?
    } else {
        first
    };
    if !matches!(
        literal.kind(),
        "string" | "string_literal" | "interpreted_string_literal" | "template_string"
    ) {
        return None;
    }
    let text = &source[literal.byte_range()];
    if text.starts_with(['f', 'F']) {
        return None;
    }
    Some(text)
}

fn fetch_method(node: tree_sitter::Node<'_>, source: &str) -> Option<String> {
    let root = arguments(node)?;
    let mut cursor = root.walk();
    let args = root
        .named_children(&mut cursor)
        .filter(|node| !node.kind().contains("comment"))
        .collect::<Vec<_>>();
    let Some(options) = args.get(1) else {
        return Some("GET".into());
    };
    if options.kind() != "object" {
        return None;
    }
    let mut cursor = options.walk();
    let mut method = None;
    for pair in options.named_children(&mut cursor) {
        if pair.kind().contains("comment") {
            continue;
        }
        if pair.kind() != "pair" {
            return None;
        }
        let key = pair.child_by_field_name("key")?;
        if source[key.byte_range()].trim_matches(['\'', '"']) == "method" {
            if method.is_some() {
                return None;
            }
            let value = pair.child_by_field_name("value")?;
            if value.kind() != "string" {
                return None;
            }
            let value = source[value.byte_range()]
                .trim_matches(['\'', '"'])
                .to_ascii_uppercase();
            if !["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
                .contains(&value.as_str())
            {
                return None;
            }
            method = Some(value);
        }
    }
    Some(method.unwrap_or_else(|| "GET".into()))
}

fn collect_refs(value: &serde_json::Value, refs: &mut BTreeSet<String>) {
    if refs.len() >= 64 {
        return;
    }
    match value {
        serde_json::Value::Object(fields) => {
            if let Some(reference) = fields
                .get("$ref")
                .and_then(|value| value.as_str())
                .and_then(|value| value.strip_prefix("#/components/schemas/"))
            {
                refs.insert(reference.to_string());
            }
            for value in fields.values() {
                collect_refs(value, refs);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_refs(value, refs);
            }
        }
        _ => {}
    }
}
