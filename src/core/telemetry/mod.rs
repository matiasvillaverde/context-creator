//! Telemetry processing module for OpenTelemetry data enrichment

pub mod correlator;
pub mod enricher;
pub mod otlp_parser;
pub mod types;

pub use correlator::{CorrelationKey, CorrelationResult, TelemetryCorrelator};
pub use enricher::TelemetryEnricher;
pub use otlp_parser::{JsonParser, OtlpParser, ProtobufParser};
pub use types::*;
