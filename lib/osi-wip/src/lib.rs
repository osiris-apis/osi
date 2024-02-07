//! # Osiris Work in Progress Library
//!
//! This library contains experimental modules of Osiris. None of them are
//! meant for production use.

#![no_std]

extern crate alloc;
extern crate core;

#[cfg(any(test, feature = "std"))]
extern crate std;

pub use osi_derive as dd;
pub mod ffi;

use osi_lib as lib;
