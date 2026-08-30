//! Unified checkpoint/restore trait for streaming indicator state persistence.
//!
//! Provides `CheckpointState` for binary serialization/deserialization of indicator
//! state, enabling pause/resume workflows and distributed processing.

#[cfg(feature = "serde")]
use serde::{de::DeserializeOwned, Serialize};

/// Errors that can occur during checkpoint operations.
#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    #[error("serialization failed: {0}")]
    SerializeFailed(String),
    #[error("deserialization failed: {0}")]
    DeserializeFailed(String),
}

/// Unified state checkpoint trait for streaming indicators.
///
/// Enables saving and restoring indicator state as opaque byte buffers,
/// supporting pause/resume, distributed fan-out, and crash recovery.
///
/// # Example
///
/// ```
/// use finkit::streaming::indicators::StreamingSma;
/// use finkit::streaming::{StreamingIndicator, CheckpointState};
///
/// let mut sma = StreamingSma::new(5);
/// for i in 0..10 { sma.next(i as f64); }
///
/// let bytes = sma.save_state().unwrap();
/// let mut restored = StreamingSma::restore_state(&bytes).unwrap();
///
/// // Both produce identical output going forward
/// assert_eq!(sma.next(10.0), restored.next(10.0));
/// ```
#[cfg(feature = "serde")]
pub trait CheckpointState: Serialize + DeserializeOwned + Sized {
    /// Serialize the current indicator state to a byte vector (bincode format).
    fn save_state(&self) -> Result<Vec<u8>, CheckpointError> {
        bincode::serialize(self).map_err(|e| CheckpointError::SerializeFailed(e.to_string()))
    }

    /// Restore an indicator from a previously saved byte buffer.
    fn restore_state(bytes: &[u8]) -> Result<Self, CheckpointError> {
        bincode::deserialize(bytes).map_err(|e| CheckpointError::DeserializeFailed(e.to_string()))
    }

    /// Hint for the expected serialized size in bytes (useful for pre-allocation).
    /// Returns 0 if unknown.
    fn state_size_hint(&self) -> usize {
        bincode::serialized_size(self).unwrap_or(0) as usize
    }
}

/// Blanket implementation: any type with Serialize + DeserializeOwned gets CheckpointState for free.
#[cfg(feature = "serde")]
impl<T> CheckpointState for T where T: Serialize + DeserializeOwned + Sized {}
