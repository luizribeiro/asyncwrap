//! # asyncwrap
//!
//! Auto-generate async wrappers for blocking code via proc macros.
//!
//! This crate provides two main macros:
//! - `#[blocking_impl(AsyncType)]` - Processes an impl block and generates async wrappers
//! - `#[async_wrap]` - Marks individual methods for async wrapper generation
//!
//! # Example
//!
//! ```ignore
//! use asyncwrap::{blocking_impl, async_wrap};
//! use std::sync::Arc;
//!
//! struct BlockingClient;
//!
//! #[blocking_impl(AsyncClient)]
//! impl BlockingClient {
//!     #[async_wrap]
//!     pub fn fetch(&self, id: u32) -> Result<String, std::io::Error> {
//!         Ok(format!("data-{id}"))
//!     }
//! }
//!
//! pub struct AsyncClient {
//!     inner: Arc<BlockingClient>,
//! }
//! ```

pub use asyncwrap_macros::{async_wrap, blocking_impl};

/// Error type for async wrapper operations.
///
/// This wraps the original error type from the blocking method and adds
/// the possibility of a task failure (panic or cancellation).
#[derive(Debug)]
pub enum AsyncWrapError<E> {
    /// The underlying blocking operation failed
    Inner(E),
    /// The spawned task was cancelled or panicked
    TaskFailed(tokio::task::JoinError),
}

/// Result type alias for methods that return `Result<T, E>`.
///
/// The async wrapper transforms `Result<T, E>` into `Result<T, AsyncWrapError<E>>`.
pub type AsyncWrapResult<R> =
    std::result::Result<<R as ResultType>::Ok, AsyncWrapError<<R as ResultType>::Err>>;

/// Helper trait to extract Ok and Err types from Result.
pub trait ResultType {
    type Ok;
    type Err;
}

impl<T, E> ResultType for std::result::Result<T, E> {
    type Ok = T;
    type Err = E;
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
