//! # Osiris Cross-Platform APIs
//!
//! This is a cross-platform implementation of the Osiris APIs. The platform
//! dependent implementations can be found in their respective backend crates.
//! This crate provides a uniform API over all the supported platforms, with
//! platform-specific opt-ins where detailed API access might be required.

#![cfg(
        any(
            target_os = "linux",
            target_os = "windows",
        )
    )]

#[cfg(target_os = "linux")]
pub(crate) use osiris_linux as native;
#[cfg(target_os = "windows")]
pub(crate) use osiris_windows as native;

pub mod application;
