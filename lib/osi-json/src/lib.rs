//! # Standalone JSON Streaming Implementation
//!
//! This crate implements a standalone JSON streaming engine as well as a
//! parser into an in-memory representation.
//!
//! ## Compatibility
//!
//! The JSON specification is quite lenient regarding allowed escape sequences
//! in JSON Strings. Effectively, JSON Strings can encode data that cannot be
//! represented in `UTF-8` or `UTF-16` (in particular, it allows unpaired
//! Unicode Surrogates). Fortunately, the specification notes that
//! implementations are free to reject any such input [^rfc_surrogate]. This
//! crate opts to do so and rejects any JSON Strings that encode anything but
//! valid Unicode Scalar Values [^unicode_scalar].
//!
//! [^unicode_scalar]: <https://www.unicode.org/glossary/#unicode_scalar_value>
//! [^rfc_surrogate]: <https://datatracker.ietf.org/doc/html/rfc8259#section-8.2>

#![no_std]

#[cfg(test)]
extern crate std;

extern crate alloc;
extern crate core;

pub mod token;
