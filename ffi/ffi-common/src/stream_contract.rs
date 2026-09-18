//! Shared provenance metadata for portable streaming checkpoints.

use serde_json::{Map, Value};

/// Validate that a checkpoint belongs to the requested logical stream.
///
/// A numerical state signature proves the formula/plan identity, but it does
/// not prove that the state came from the same symbol, timeframe, or input
/// revision. The transport contract therefore requires both fields whenever a
/// checkpoint crosses a language boundary.
pub(crate) fn validate_checkpoint_metadata(
    checkpoint: &Value,
    scope: &str,
    data_revision: u64,
) -> Result<(), String> {
    let object = checkpoint
        .as_object()
        .ok_or_else(|| "stream checkpoint must be a JSON object".to_string())?;
    let checkpoint_scope = object
        .get("scope")
        .and_then(Value::as_str)
        .ok_or_else(|| "stream checkpoint scope is required".to_string())?;
    let checkpoint_revision = object
        .get("data_revision")
        .and_then(Value::as_u64)
        .ok_or_else(|| "stream checkpoint data_revision is required".to_string())?;
    if checkpoint_scope != scope {
        return Err(format!(
            "stream checkpoint scope mismatch: expected {scope}, got {checkpoint_scope}"
        ));
    }
    if checkpoint_revision != data_revision {
        return Err(format!(
            "stream checkpoint data_revision mismatch: expected {data_revision}, got {checkpoint_revision}"
        ));
    }
    Ok(())
}

/// Add stream provenance metadata to a core checkpoint object.
pub(crate) fn annotate_checkpoint(
    checkpoint: Value,
    scope: &str,
    data_revision: u64,
) -> Result<Value, String> {
    let mut object: Map<String, Value> = checkpoint
        .as_object()
        .cloned()
        .ok_or_else(|| "stream checkpoint must be a JSON object".to_string())?;
    object.insert("scope".to_string(), Value::String(scope.to_string()));
    object.insert("data_revision".to_string(), Value::from(data_revision));
    Ok(Value::Object(object))
}
