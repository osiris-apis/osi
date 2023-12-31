//! # Osiris Shared Library
//!
//! This crate provides shared utility functions used across many
//! Osiris crates.

pub mod align;
pub mod error;
pub mod ffi;
pub mod hash;
pub mod hmac;
pub mod str;

pub use osi_derive as dd;
