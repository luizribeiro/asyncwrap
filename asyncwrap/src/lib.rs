//! # asyncwrap
//!
//! Auto-generate async wrappers for blocking code via proc macros.
//!
//! See the [SPEC.md](../SPEC.md) for full documentation on how to implement this crate.

pub use asyncwrap_macros::{async_wrap, blocking_impl};

/// Error type for async wrapper operations
#[derive(Debug)]
pub enum AsyncWrapError<E> {
    /// The underlying blocking operation failed
    Inner(E),
    /// The spawned task was cancelled or panicked
    TaskFailed(tokio::task::JoinError),
}

impl<E> From<tokio::task::JoinError> for AsyncWrapError<E> {
    fn from(err: tokio::task::JoinError) -> Self {
        AsyncWrapError::TaskFailed(err)
    }
}

impl<E: std::fmt::Display> std::fmt::Display for AsyncWrapError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AsyncWrapError::Inner(e) => write!(f, "{e}"),
            AsyncWrapError::TaskFailed(e) => write!(f, "async task failed: {e}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for AsyncWrapError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AsyncWrapError::Inner(e) => Some(e),
            AsyncWrapError::TaskFailed(e) => Some(e),
        }
    }
}
