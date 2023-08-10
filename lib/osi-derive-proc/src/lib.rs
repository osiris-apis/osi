//! # Direct Derive Proc Macro
//!
//! This proc-macro crate provides an alternative
//! [`crate::derive`](macro@crate::derive) attribute, which generates trait
//! implementations for a type based solely on its structural layout. It works
//! very similar to the [`core::derive`](core::prelude::v1::derive) attribute
//! of the Rust standard library, but uses direct trait-bounds on each field
//! rather than putting a trait-bound on each generic argument (known as
//! _perfect derive_).
//!
//! The attribute has builtin support to derive many of the core Rust traits,
//! but allows for external code to derive any custom trait via simple Rust
//! macro definitions.

use ::proc_macro;
use ::syn;

mod derive;

/// ## Direct Derive
///
/// This attribute is an alternative to
/// [`core::derive`](core::prelude::v1::derive) of the Rust standard library.
/// Unlike the standard version, this attribute places trait bounds on the
/// type of each field, rather than on each generic parameter.
///
/// This attribute takes a list of traits to derive. Note that these arguments
/// do not directly refer to the trait to derive, but to a macro that
/// implements the derive-operation for a given trait. Derive-operations for
/// the most common core traits are bundled with the macro.
///
/// ### Limitations
///
/// - Only `struct`-types are supported as targets. Support for `enum`-types
///   and more might be added later.
///
/// ### Custom Trait Support
///
/// The arguments to the attribute actually resolve to a Rust macro that
/// provides the derive-implementation for a given trait. This
/// allows external crates to add derive-support for any possible trait.
/// Integration for the most common traits of the Rust standard library is
/// bundled with this attribute. These can be used directly, without requiring
/// integration as described in this chapter.
///
/// To add support for a new trait, a macro must be defined, which is then
/// passed to the attribute. This will instantiate an
/// invocation of this macro and pass the following information as arguments:
///
/// - **kind**: The kind of macro invocation. This can be one of:
///     - **derive_struct**: The target type is a struct with named fields.
///     - **derive_tuple**: The target type is a tuple with unnamed fields.
/// - **ident**: The identifier of the target type this is derived on.
/// - **type**: The fully-qualified type of the target type.
/// - **generics**: The generics required for the target type, enclosed in
///     angle brackets including their trait bounds. The entire block is
///     surrounded by parentheses, and may be empty if no generics are used.
/// - **where**: A comma-separated list of all where-clauses required for the
///     target type. Each where-clause is enclosed in parentheses, and the
///     entire argument is itself enclosed in parentheses.
/// - **field-idents**: A comma-separated list of all field identifiers of the
///     target type. The entire argument is itself enclosed in parentheses.
/// - **field-types**: A comma-separated list of all field types of the target
///     type. The entire argument is itself enclosed in parentheses.
#[proc_macro_attribute]
pub fn derive(
    attributes: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let syn_list = syn::parse_macro_input!(attributes as derive::DeriveList);
    let syn_item = syn::parse_macro_input!(item as derive::DeriveItem);

    proc_macro::TokenStream::from(derive::derive(syn_list, syn_item))
}
