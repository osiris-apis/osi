//! # Operating System Interfaces
//!
//! This crate provides direct access to operating system interfaces from Rust.

// # Development Documentation
//
// This comment contains information on the development of this crate, desired
// Rust features, and possible future enhancements.
//
// ## Desired Rust Features
//
// - Const Fn Traits (#67792): Currently, we cannot instantiate
//   trait-associated types in constant-expressions, because all that is known
//   about them is their trait-bounds, and those currently do not support
//   `const fn` methods. Our workaround is to provide a `constant()` function
//   on each implementing type, and thus requiring all downstream users to
//   operate on the types rather than the traits, if they need this helper.
//
// - Inherent Associated Types (#8995): Unlike traits, structs currently cannot
//   have associated types. If this was supported, we could use generic structs
//   rather than traits to back Api abstractions like `ffi::util::Abi`. It
//   would all the downsides of traits and instead provide actual types.
//
// - Trait Aliases (#41517): The `osi-derive-proc` crate currently uses macro
//   dispatchers to refer to derive-operators. We would much rather refer to
//   module names (or traits, or structs), which then provide all required
//   parameters of a derive-operator. However, we need to parameterize the
//   operator by the trait it is meant to implement, and Rust currently does
//   not allow to define trait aliases in any way.
//
// - Macro in Trait Position (<missing>): It is currently no allowed to
//   evaluate macros in a position where a trait is expected. This would be
//   a suitable alternative to trait-aliases and simplify the macro based
//   dispatchers of `osi-derive-proc`.
//
// - Const-expressions in `repr(align(...))` (<missing>): It is currently not
//   allowed to have constant-expressions in `repr(align(...))`, hence one
//   cannot propagate alignments between types. Instead, we have to use
//   phantom-markers from `mem::align` to achieve a similar result.
//
// - Guaranteed Nonnull-optimization (`rustc_nonnull_optimization_guaranteed`):
//   There is no way to guarantee the nonnull-optimization rust uses for its
//   builtin type. While we rely on its presence, and it is unlikely to break,
//   the compiler does not guarantee it.

#![no_std]

#[cfg(test)]
extern crate std;

pub use osi_apis as apis;
pub use osi_derive as dd;
pub use osi_lib as lib;
