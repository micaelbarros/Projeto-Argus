//! NS-007, NS-008 & NS-009: Latency validation and statistics helpers.

use argus_core::errors::{ArgusError, ArgusResult};

/// Validates target host input before executing latency probes (prevents empty/shell-like input).
pub fn validate_latency_target(target: &str) -> ArgusResult<String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(ArgusError::NetworkUnavailable(
            "Destino de latência não pode ser vazio.".to_string(),
        ));
    }
    if trimmed.len() > 253
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'))
    {
        return Err(ArgusError::NetworkUnavailable(format!(
            "Destino de latência inválido: '{trimmed}'"
        )));
    }
    Ok(trimmed.to_string())
}
