//! Proc macros for asyncwrap
//!
//! This crate provides two main macros:
//! - `#[async_wrap]` - Marks a method for async wrapper generation
//! - `#[blocking_impl(AsyncType)]` - Processes an impl block and generates async wrappers

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, FnArg, Ident, ImplItem, ImplItemFn, ItemImpl, Pat, ReturnType, Token, Type,
    Visibility,
};

#[derive(Clone, Copy, Default)]
enum Strategy {
    #[default]
    SpawnBlocking,
    BlockInPlace,
}

/// Marks a method for async wrapper generation.
///
/// This attribute should be placed on public methods within a `#[blocking_impl]` block.
/// The method will have an async version generated in the corresponding async wrapper struct.
///
/// # Requirements
///
/// - The method must take `&self` (not `&mut self` or `self`)
/// - All arguments must be `Send + 'static` to cross the `spawn_blocking` boundary
/// - The method must not be async
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
    let Ok(method) = syn::parse::<ImplItemFn>(item.clone()) else {
        return item;
    };

    if let Err(e) = validate_async_wrap_method(&method) {
        return e.to_compile_error().into();
    }

    item
}

struct BlockingImplArgs {
    async_type: Type,
    strategy: Strategy,
}

impl Parse for BlockingImplArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let async_type: Type = input.parse()?;

        let mut strategy = Strategy::default();

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let ident: Ident = input.parse()?;
            if ident != "strategy" {
                return Err(syn::Error::new_spanned(ident, "expected `strategy`"));
            }
            input.parse::<Token![=]>()?;
            let value: syn::LitStr = input.parse()?;
            strategy = match value.value().as_str() {
                "spawn_blocking" => Strategy::SpawnBlocking,
                "block_in_place" => Strategy::BlockInPlace,
                other => {
                    return Err(syn::Error::new_spanned(
                        value,
                        format!(
                            "unknown strategy \"{other}\", expected \"spawn_blocking\" or \"block_in_place\""
                        ),
                    ))
                }
            };
        }

        Ok(BlockingImplArgs {
            async_type,
            strategy,
        })
    }
}

struct MethodInfo {
    name: Ident,
    visibility: Visibility,
    args: Vec<(Ident, Type)>,
    return_type: Option<Type>,
    is_result: bool,
    doc_attrs: Vec<syn::Attribute>,
}

fn has_async_wrap_attr(method: &ImplItemFn) -> bool {
    method
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("async_wrap"))
}

fn remove_async_wrap_attr(method: &mut ImplItemFn) {
    method
        .attrs
        .retain(|attr| !attr.path().is_ident("async_wrap"));
}

fn is_self_by_ref(arg: &FnArg) -> bool {
    matches!(arg, FnArg::Receiver(r) if r.reference.is_some() && r.mutability.is_none())
}

fn validate_async_wrap_method(method: &ImplItemFn) -> syn::Result<()> {
    if method.sig.asyncness.is_some() {
        return Err(syn::Error::new_spanned(
            method.sig.asyncness,
            "#[async_wrap] cannot be used on async methods",
        ));
    }

    match method.sig.inputs.first() {
        Some(arg) if is_self_by_ref(arg) => Ok(()),
        Some(FnArg::Receiver(r)) if r.mutability.is_some() => Err(syn::Error::new_spanned(
            r,
            "#[async_wrap] requires `&self`, not `&mut self`",
        )),
        Some(FnArg::Receiver(r)) if r.reference.is_none() => Err(syn::Error::new_spanned(
            r,
            "#[async_wrap] requires `&self`, not `self`",
        )),
        Some(arg) => Err(syn::Error::new_spanned(
            arg,
            "#[async_wrap] requires methods taking `&self`",
        )),
        None => Err(syn::Error::new_spanned(
            &method.sig,
            "#[async_wrap] requires methods taking `&self`",
        )),
    }
}

fn is_result_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Result";
        }
    }
    false
}

fn extract_method_info(method: &ImplItemFn) -> Option<MethodInfo> {
    let first_arg = method.sig.inputs.first()?;
    if !is_self_by_ref(first_arg) {
        return None;
    }

    let name = method.sig.ident.clone();
    let visibility = method.vis.clone();

    let args: Vec<(Ident, Type)> = method
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    return Some((pat_ident.ident.clone(), (*pat_type.ty).clone()));
                }
            }
            None
        })
        .collect();

    let (return_type, is_result) = match &method.sig.output {
        ReturnType::Default => (None, false),
        ReturnType::Type(_, ty) => (Some((**ty).clone()), is_result_type(ty)),
    };

    let doc_attrs: Vec<_> = method
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .cloned()
        .collect();

    Some(MethodInfo {
        name,
        visibility,
        args,
        return_type,
        is_result,
        doc_attrs,
    })
}

fn generate_async_method(info: &MethodInfo, strategy: Strategy) -> TokenStream2 {
    let name = &info.name;
    let vis = &info.visibility;
    let doc_attrs = &info.doc_attrs;
    let arg_names: Vec<_> = info.args.iter().map(|(name, _)| name).collect();
    let arg_types: Vec<_> = info.args.iter().map(|(_, ty)| ty).collect();

    match strategy {
        Strategy::SpawnBlocking => {
            let spawn_call = quote! {
                let inner = ::std::sync::Arc::clone(&self.inner);
                ::tokio::task::spawn_blocking(move || inner.#name(#(#arg_names),*)).await
            };

            let (return_type, body) = if info.is_result {
                let inner_return = info.return_type.as_ref().unwrap();
                (
                    quote! { -> ::asyncwrap::AsyncWrapResult<#inner_return> },
                    quote! {
                        #spawn_call
                            .map_err(::asyncwrap::AsyncWrapError::TaskFailed)?
                            .map_err(::asyncwrap::AsyncWrapError::Inner)
                    },
                )
            } else {
                let ret_ty = info
                    .return_type
                    .as_ref()
                    .map_or_else(|| quote! { () }, |ty| quote! { #ty });
                (
                    quote! { -> ::core::result::Result<#ret_ty, ::tokio::task::JoinError> },
                    spawn_call,
                )
            };

            quote! {
                #(#doc_attrs)*
                #vis async fn #name(&self, #(#arg_names: #arg_types),*) #return_type {
                    #body
                }
            }
        }
        Strategy::BlockInPlace => {
            let return_type = info
                .return_type
                .as_ref()
                .map(|ty| quote! { -> #ty });

            quote! {
                #(#doc_attrs)*
                #vis async fn #name(&self, #(#arg_names: #arg_types),*) #return_type {
                    ::tokio::task::block_in_place(|| self.inner.#name(#(#arg_names),*))
                }
            }
        }
    }
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
    let args = parse_macro_input!(attr as BlockingImplArgs);
    let mut input = parse_macro_input!(item as ItemImpl);

    let async_type = &args.async_type;

    let generics = &input.generics;
    let where_clause = &input.generics.where_clause;
    let generic_params: Vec<_> = generics.params.iter().collect();

    let mut async_methods = Vec::new();
    let mut errors = Vec::new();

    for item in &mut input.items {
        if let ImplItem::Fn(method) = item {
            if has_async_wrap_attr(method) {
                if let Err(e) = validate_async_wrap_method(method) {
                    errors.push(e);
                } else if let Some(info) = extract_method_info(method) {
                    async_methods.push(generate_async_method(&info, args.strategy));
                }
                remove_async_wrap_attr(method);
            }
        }
    }

    if !errors.is_empty() {
        let compile_errors = errors.into_iter().map(|e| e.to_compile_error());
        return quote! {
            #input
            #(#compile_errors)*
        }
        .into();
    }

    let async_impl = if generic_params.is_empty() {
        quote! {
            impl #async_type {
                #(#async_methods)*
            }
        }
    } else {
        quote! {
            impl<#(#generic_params),*> #async_type #where_clause {
                #(#async_methods)*
            }
        }
    };

    let output = quote! {
        #input
        #async_impl
    };

    output.into()
}
