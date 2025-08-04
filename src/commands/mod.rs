//! Command implementations

pub mod diff;
pub mod search;
pub mod telemetry;

pub use diff::run_diff;
pub use search::run_search;
pub use telemetry::run_telemetry;
