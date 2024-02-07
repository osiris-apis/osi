//! # Osiris Shared Library
//!
//! This crate provides shared utility functions used across many
//! Osiris crates.

#![no_std]

extern crate alloc;
extern crate core;

#[cfg(any(test, feature = "std"))]
extern crate std;

pub mod align;
pub mod args;
pub mod compat;
pub mod error;
pub mod hash;
pub mod hmac;
pub mod str;

pub use osi_derive as dd;
pub use osi_json as json;
