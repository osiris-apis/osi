//! # Osiris Build System
//!
//! The Osiris Build System integrates Rust applications into a wide range
//! of target platforms, including mobile platforms like Android and iOS, as
//! well as desktop platforms like Linux and Windows, or custom platform
//! targets. The build system is a standalone integration effort, not
//! requiring other Osiris modules to be used, nor placing any restrictions
//! on the Rust application.
//!
//! The build system bundles Rust applications into the respective application
//! format of a target platform. Platform integration can be under full
//! control of the Rust application, allowing direct access to the native
//! application build process of each platform. Alternatively, the platform
//! integration can be left under control of the build system, thus hiding the
//! entire native integration and instead using the provided abstractions.

pub mod cargo;
pub mod config;
pub mod toml;
pub mod util;
