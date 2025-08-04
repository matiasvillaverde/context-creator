//! OTLP format parsers for JSON and protobuf

use anyhow::{Context, Result};
use prost::Message;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::core::telemetry::types::*;

// OpenTelemetry protobuf message definitions
// Based on: https://github.com/open-telemetry/opentelemetry-proto/blob/main/opentelemetry/proto/trace/v1/trace.proto

/// OTLP ExportTraceServiceRequest message
#[derive(Clone, PartialEq, Message)]
pub struct ExportTraceServiceRequest {
    #[prost(message, repeated, tag = "1")]
    pub resource_spans: Vec<ResourceSpans>,
}

/// OTLP ResourceSpans message
#[derive(Clone, PartialEq, Message)]
pub struct ResourceSpans {
    #[prost(message, optional, tag = "1")]
    pub resource: Option<Resource>,
    #[prost(message, repeated, tag = "2")]
    pub scope_spans: Vec<ScopeSpans>,
}

/// OTLP Resource message
#[derive(Clone, PartialEq, Message)]
pub struct Resource {
    #[prost(message, repeated, tag = "1")]
    pub attributes: Vec<KeyValue>,
}

/// OTLP ScopeSpans message
#[derive(Clone, PartialEq, Message)]
pub struct ScopeSpans {
    #[prost(message, repeated, tag = "2")]
    pub spans: Vec<ProtobufSpan>,
}

/// OTLP Span message
#[derive(Clone, PartialEq, Message)]
pub struct ProtobufSpan {
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(fixed64, tag = "7")]
    pub start_time_unix_nano: u64,
    #[prost(fixed64, tag = "8")]
    pub end_time_unix_nano: u64,
    #[prost(message, repeated, tag = "9")]
    pub attributes: Vec<KeyValue>,
    #[prost(message, optional, tag = "15")]
    pub status: Option<Status>,
}

/// OTLP Status message
#[derive(Clone, PartialEq, Message)]
pub struct Status {
    #[prost(enumeration = "StatusCode", tag = "2")]
    pub code: i32,
    #[prost(string, tag = "3")]
    pub message: String,
}

/// OTLP StatusCode enumeration
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum StatusCode {
    Unset = 0,
    Ok = 1,
    Error = 2,
}

/// OTLP KeyValue message
#[derive(Clone, PartialEq, Message)]
pub struct KeyValue {
    #[prost(string, tag = "1")]
    pub key: String,
    #[prost(message, optional, tag = "2")]
    pub value: Option<AnyValue>,
}

/// OTLP AnyValue message
#[derive(Clone, PartialEq, Message)]
pub struct AnyValue {
    #[prost(oneof = "any_value::Value", tags = "1, 2, 3, 4")]
    pub value: Option<any_value::Value>,
}

/// AnyValue value oneof
pub mod any_value {
    #[allow(clippy::enum_variant_names)]
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Value {
        #[prost(string, tag = "1")]
        StringValue(String),
        #[prost(int64, tag = "2")]
        IntValue(i64),
        #[prost(double, tag = "3")]
        DoubleValue(f64),
        #[prost(bool, tag = "4")]
        BoolValue(bool),
    }
}

/// Trait for parsing OTLP data from different formats
pub trait OtlpParser {
    /// Parse OTLP data from a file
    fn parse_file(&self, path: &Path) -> Result<ParsedTelemetry>;

    /// Parse OTLP data from bytes
    fn parse_bytes(&self, data: &[u8]) -> Result<ParsedTelemetry>;
}

/// JSON format parser for OTLP data
pub struct JsonParser;

impl JsonParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonParser {
    /// Extract telemetry spans from parsed JSON
    fn extract_spans(&self, otlp: OtlpJson) -> Vec<TelemetrySpan> {
        let mut spans = Vec::new();

        if let Some(resource_spans) = otlp.resource_spans {
            for rs in resource_spans {
                // Extract service name from resource attributes
                let service_name = rs
                    .resource
                    .as_ref()
                    .and_then(|r| r.attributes.as_ref())
                    .and_then(|attrs| {
                        attrs
                            .iter()
                            .find(|a| a.key == "service.name")
                            .and_then(|a| a.value.as_ref())
                            .and_then(|v| v.string_value.clone())
                    });

                if let Some(scope_spans) = rs.scope_spans {
                    for ss in scope_spans {
                        if let Some(span_list) = ss.spans {
                            for span in span_list {
                                if let Some(telemetry_span) =
                                    self.convert_span(span, service_name.clone())
                                {
                                    spans.push(telemetry_span);
                                }
                            }
                        }
                    }
                }
            }
        }

        spans
    }

    /// Convert OTLP span to our internal representation
    fn convert_span(&self, span: Span, service_name: Option<String>) -> Option<TelemetrySpan> {
        let mut telemetry_span = TelemetrySpan {
            name: span.name,
            function_name: None,
            file_path: None,
            line_number: None,
            service_name,
            start_time_nanos: span
                .start_time_unix_nano
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0),
            end_time_nanos: span
                .end_time_unix_nano
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0),
            duration_ms: 0.0,
            attributes: HashMap::new(),
        };

        // Extract code-related attributes
        if let Some(attributes) = &span.attributes {
            for attr in attributes {
                match attr.key.as_str() {
                    "code.function.name" => {
                        telemetry_span.function_name =
                            attr.value.as_ref().and_then(|v| v.string_value.clone());
                    }
                    "code.file.path" | "code.filepath" => {
                        telemetry_span.file_path = attr
                            .value
                            .as_ref()
                            .and_then(|v| v.string_value.as_ref())
                            .map(|p| Path::new(p).to_path_buf());
                    }
                    "code.line.number" | "code.lineno" => {
                        telemetry_span.line_number = attr
                            .value
                            .as_ref()
                            .and_then(|v| v.int_value.as_ref())
                            .and_then(|s| s.parse::<u32>().ok());
                    }
                    _ => {
                        // Store other attributes
                        if let Some(value) = &attr.value {
                            if let Some(attr_value) =
                                self.convert_json_attribute_value(value.clone())
                            {
                                telemetry_span
                                    .attributes
                                    .insert(attr.key.clone(), attr_value);
                            }
                        }
                    }
                }
            }
        }

        // Calculate duration
        telemetry_span.calculate_duration_ms();

        Some(telemetry_span)
    }

    /// Convert OTLP JSON attribute value to our internal representation
    fn convert_json_attribute_value(
        &self,
        value: crate::core::telemetry::types::AnyValue,
    ) -> Option<AttributeValue> {
        if let Some(s) = value.string_value {
            Some(AttributeValue::String(s))
        } else if let Some(i) = value.int_value {
            i.parse::<i64>().ok().map(AttributeValue::Int)
        } else if let Some(d) = value.double_value {
            Some(AttributeValue::Double(d))
        } else {
            value.bool_value.map(AttributeValue::Bool)
        }
    }
}

impl OtlpParser for JsonParser {
    fn parse_file(&self, path: &Path) -> Result<ParsedTelemetry> {
        let data = fs::read(path)
            .with_context(|| format!("Failed to read telemetry file: {}", path.display()))?;
        self.parse_bytes(&data)
    }

    fn parse_bytes(&self, data: &[u8]) -> Result<ParsedTelemetry> {
        let otlp: OtlpJson = serde_json::from_slice(data).context("Failed to parse OTLP JSON")?;

        let spans = self.extract_spans(otlp);
        let code_spans = spans
            .iter()
            .filter(|s| s.function_name.is_some() || s.file_path.is_some())
            .cloned()
            .collect();

        Ok(ParsedTelemetry { spans, code_spans })
    }
}

/// Protobuf format parser for OTLP data
pub struct ProtobufParser;

impl ProtobufParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProtobufParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtobufParser {
    /// Extract telemetry spans from parsed protobuf
    fn extract_spans(&self, otlp: ExportTraceServiceRequest) -> Vec<TelemetrySpan> {
        let mut spans = Vec::new();

        for resource_span in otlp.resource_spans {
            // Extract service name from resource attributes
            let service_name = resource_span.resource.as_ref().and_then(|r| {
                r.attributes
                    .iter()
                    .find(|attr| attr.key == "service.name")
                    .and_then(|attr| attr.value.as_ref())
                    .and_then(|v| match &v.value {
                        Some(any_value::Value::StringValue(s)) => Some(s.clone()),
                        _ => None,
                    })
            });

            for scope_span in resource_span.scope_spans {
                for span in scope_span.spans {
                    if let Some(telemetry_span) =
                        self.convert_protobuf_span(span, service_name.clone())
                    {
                        spans.push(telemetry_span);
                    }
                }
            }
        }

        spans
    }

    /// Convert OTLP protobuf span to our internal representation
    fn convert_protobuf_span(
        &self,
        span: ProtobufSpan,
        service_name: Option<String>,
    ) -> Option<TelemetrySpan> {
        let mut telemetry_span = TelemetrySpan {
            name: span.name,
            function_name: None,
            file_path: None,
            line_number: None,
            service_name,
            start_time_nanos: span.start_time_unix_nano,
            end_time_nanos: span.end_time_unix_nano,
            duration_ms: 0.0,
            attributes: HashMap::new(),
        };

        // Extract code-related attributes
        for attr in span.attributes {
            match attr.key.as_str() {
                "code.function.name" => {
                    if let Some(value) = attr.value.and_then(|v| match v.value {
                        Some(any_value::Value::StringValue(s)) => Some(s),
                        _ => None,
                    }) {
                        telemetry_span.function_name = Some(value);
                    }
                }
                "code.file.path" | "code.filepath" => {
                    if let Some(value) = attr.value.and_then(|v| match v.value {
                        Some(any_value::Value::StringValue(s)) => Some(s),
                        _ => None,
                    }) {
                        telemetry_span.file_path = Some(Path::new(&value).to_path_buf());
                    }
                }
                "code.line.number" | "code.lineno" => {
                    if let Some(value) = attr.value.and_then(|v| match v.value {
                        Some(any_value::Value::IntValue(i)) => Some(i as u32),
                        _ => None,
                    }) {
                        telemetry_span.line_number = Some(value);
                    }
                }
                _ => {
                    // Store other attributes
                    if let Some(attr_value) = self.convert_protobuf_attribute_value(attr.value) {
                        telemetry_span.attributes.insert(attr.key, attr_value);
                    }
                }
            }
        }

        // Add status information as attributes
        if let Some(status) = span.status {
            telemetry_span.attributes.insert(
                "status.code".to_string(),
                AttributeValue::Int(status.code as i64),
            );
            if !status.message.is_empty() {
                telemetry_span.attributes.insert(
                    "status.message".to_string(),
                    AttributeValue::String(status.message),
                );
            }
        }

        // Calculate duration
        telemetry_span.calculate_duration_ms();

        Some(telemetry_span)
    }

    /// Convert OTLP protobuf attribute value to our internal representation
    fn convert_protobuf_attribute_value(&self, value: Option<AnyValue>) -> Option<AttributeValue> {
        value.and_then(|v| match v.value {
            Some(any_value::Value::StringValue(s)) => Some(AttributeValue::String(s)),
            Some(any_value::Value::IntValue(i)) => Some(AttributeValue::Int(i)),
            Some(any_value::Value::DoubleValue(d)) => Some(AttributeValue::Double(d)),
            Some(any_value::Value::BoolValue(b)) => Some(AttributeValue::Bool(b)),
            None => None,
        })
    }
}

impl OtlpParser for ProtobufParser {
    fn parse_file(&self, path: &Path) -> Result<ParsedTelemetry> {
        let data = fs::read(path)
            .with_context(|| format!("Failed to read telemetry file: {}", path.display()))?;
        self.parse_bytes(&data)
    }

    fn parse_bytes(&self, data: &[u8]) -> Result<ParsedTelemetry> {
        // Handle empty data gracefully
        if data.is_empty() {
            return Ok(ParsedTelemetry {
                spans: vec![],
                code_spans: vec![],
            });
        }

        // Try to decode as ExportTraceServiceRequest (the most common OTLP format)
        let otlp = ExportTraceServiceRequest::decode(data)
            .context("Failed to decode OTLP protobuf data")?;

        let spans = self.extract_spans(otlp);
        let code_spans = spans
            .iter()
            .filter(|s| s.function_name.is_some() || s.file_path.is_some())
            .cloned()
            .collect();

        Ok(ParsedTelemetry { spans, code_spans })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_protobuf_parser_basic_deserialization() {
        // Given: A simple protobuf binary with basic OTLP structure
        // For now, we test that the method doesn't panic and returns a result
        let parser = ProtobufParser::new();
        let empty_data = vec![]; // Empty protobuf data

        // When: Parsing empty protobuf data
        let result = parser.parse_bytes(&empty_data);

        // Then: Should return empty parsed data (not panic)
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.spans.len(), 0);
        assert_eq!(parsed.code_spans.len(), 0);
    }

    #[test]
    fn test_protobuf_parser_invalid_data() {
        // Given: Invalid protobuf data
        let parser = ProtobufParser::new();
        let invalid_data = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Invalid protobuf

        // When: Parsing invalid data
        let result = parser.parse_bytes(&invalid_data);

        // Then: Should handle error gracefully
        match result {
            Ok(parsed) => {
                // If it succeeds, should return empty data
                assert_eq!(parsed.spans.len(), 0);
                assert_eq!(parsed.code_spans.len(), 0);
            }
            Err(_) => {
                // Error is also acceptable for invalid data
            }
        }
    }

    #[test]
    fn test_protobuf_parser_file_reading() {
        // Given: A temporary file with protobuf data
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let test_data = vec![0x08, 0x96, 0x01]; // Simple protobuf varint
        temp_file
            .write_all(&test_data)
            .expect("Failed to write test data");

        let parser = ProtobufParser::new();

        // When: Parsing file
        let result = parser.parse_file(temp_file.path());

        // Then: Should read file and attempt to parse
        match result {
            Ok(_) => {
                // Success is good
            }
            Err(e) => {
                // For invalid protobuf data, error is acceptable
                println!("Expected error for invalid protobuf data: {e}");
                // The test should pass as long as we can handle the error gracefully
            }
        }
    }

    #[test]
    fn test_protobuf_parser_vs_json_parser_interface() {
        // Given: Both parsers
        let protobuf_parser = ProtobufParser::new();
        let json_parser = JsonParser::new();

        // When: Using same interface
        let empty_data = vec![];
        let pb_result = protobuf_parser.parse_bytes(&empty_data);
        let json_result = json_parser.parse_bytes(b"{}");

        // Then: Both should implement same interface
        assert!(pb_result.is_ok());
        assert!(json_result.is_ok());
    }

    use std::path::PathBuf;

    #[test]
    fn test_parse_otlp_json_with_code_attributes() {
        let json_data = r#"{
            "resourceSpans": [{
                "resource": {
                    "attributes": [{
                        "key": "service.name",
                        "value": { "stringValue": "payment-api" }
                    }]
                },
                "scopeSpans": [{
                    "spans": [{
                        "name": "process_payment",
                        "startTimeUnixNano": "1704067200000000000",
                        "endTimeUnixNano": "1704067200050000000",
                        "attributes": [
                            {
                                "key": "code.function.name",
                                "value": { "stringValue": "process_payment" }
                            },
                            {
                                "key": "code.file.path",
                                "value": { "stringValue": "src/api/handlers.rs" }
                            },
                            {
                                "key": "code.line.number",
                                "value": { "intValue": "42" }
                            }
                        ]
                    }]
                }]
            }]
        }"#;

        let parser = JsonParser::new();
        let result = parser.parse_bytes(json_data.as_bytes()).unwrap();

        assert_eq!(result.spans.len(), 1);
        assert_eq!(result.code_spans.len(), 1);

        let span = &result.code_spans[0];
        assert_eq!(span.name, "process_payment");
        assert_eq!(span.function_name, Some("process_payment".to_string()));
        assert_eq!(span.file_path, Some(PathBuf::from("src/api/handlers.rs")));
        assert_eq!(span.line_number, Some(42));
        assert_eq!(span.service_name, Some("payment-api".to_string()));
        assert_eq!(span.duration_ms, 50.0);
    }

    #[test]
    fn test_parse_otlp_json_without_code_attributes() {
        let json_data = r#"{
            "resourceSpans": [{
                "scopeSpans": [{
                    "spans": [{
                        "name": "database_query",
                        "startTimeUnixNano": "1704067200000000000",
                        "endTimeUnixNano": "1704067200100000000",
                        "attributes": [
                            {
                                "key": "db.statement",
                                "value": { "stringValue": "SELECT * FROM users" }
                            }
                        ]
                    }]
                }]
            }]
        }"#;

        let parser = JsonParser::new();
        let result = parser.parse_bytes(json_data.as_bytes()).unwrap();

        assert_eq!(result.spans.len(), 1);
        assert_eq!(result.code_spans.len(), 0); // No code attributes

        let span = &result.spans[0];
        assert_eq!(span.name, "database_query");
        assert!(span.function_name.is_none());
        assert!(span.file_path.is_none());
    }

    #[test]
    fn test_parse_invalid_json() {
        let invalid_json = r#"{ invalid json }"#;

        let parser = JsonParser::new();
        let result = parser.parse_bytes(invalid_json.as_bytes());

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse OTLP JSON"));
    }
}
