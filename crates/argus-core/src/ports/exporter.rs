//! ReportExporterPort — JSON and Markdown report formatting and export.

use crate::domain::report::{ArgusReport, ReportFormat};
use crate::errors::ArgusResult;
use serde::Serialize;

/// Port abstracting report serialization into JSON or Markdown.
pub trait ReportExporterPort: Send + Sync {
    /// Renders a combined `ArgusReport` in the selected format.
    fn render_report(&self, report: &ArgusReport, format: ReportFormat) -> ArgusResult<String>;

    /// Serializes any serializable domain payload into pretty JSON.
    fn render_json_value(&self, value: &serde_json::Value) -> ArgusResult<String>;
}

/// Helper to convert any `Serialize` value into a `serde_json::Value`.
pub fn to_json_value<T: Serialize>(value: &T) -> ArgusResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|e| crate::errors::ArgusError::ExportError(e.to_string()))
}
