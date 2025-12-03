//! Proc macros for asyncwrap
//!
//! This crate provides two main macros:
//! - `#[async_wrap]` - Marks a method for async wrapper generation
//! - `#[blocking_impl(AsyncType)]` - Processes an impl block and generates async wrappers

use proc_macro::TokenStream;

/// Marks a method for async wrapper generation.
///
/// This attribute should be placed on public methods within a `#[blocking_impl]` block.
/// The method will have an async version generated in the corresponding async wrapper struct.
///
/// # Example
///
/// ```ignore
/// #[blocking_impl(AsyncClient)]
/// impl BlockingClient {
///     #[async_wrap]
///     pub fn get_data(&self) -> Result<Data, Error> {
///         // blocking implementation
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn async_wrap(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // This is a marker attribute - the actual processing happens in blocking_impl
    // We just pass through the item unchanged
    item
}

/// Processes an impl block and generates async wrappers for marked methods.
///
/// # Arguments
///
/// The attribute takes the name of the async wrapper struct as an argument.
///
/// # Example
///
/// ```ignore
/// use asyncwrap::blocking_impl;
/// use std::sync::Arc;
///
/// struct BlockingClient {
///     // fields
/// }
///
/// #[blocking_impl(AsyncClient)]
/// impl BlockingClient {
///     #[async_wrap]
///     pub fn fetch(&self, id: u32) -> Result<String, Error> {
///         // blocking implementation
///     }
/// }
///
/// // You still need to define the async struct:
/// pub struct AsyncClient {
///     inner: Arc<BlockingClient>,
/// }
///
/// // The macro generates:
/// // impl AsyncClient {
/// //     pub async fn fetch(&self, id: u32) -> Result<String, AsyncWrapError<Error>> {
/// //         let inner = Arc::clone(&self.inner);
/// //         tokio::task::spawn_blocking(move || inner.fetch(id))
/// //             .await
/// //             .map_err(AsyncWrapError::from)?
/// //     }
/// // }
/// ```
#[proc_macro_attribute]
pub fn blocking_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    // TODO: Implement this
    // 1. Parse the attribute to get the async wrapper type name
    // 2. Parse the impl block
    // 3. Find all methods marked with #[async_wrap]
    // 4. Generate the original impl (with #[async_wrap] attrs removed)
    // 5. Generate an impl block for the async wrapper type
    // 6. Return both impl blocks

    // For now, just return the original item unchanged
    let _ = attr; // TODO: use this
    item
}
