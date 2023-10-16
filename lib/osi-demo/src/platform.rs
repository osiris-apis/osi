//! Platform Layer
//!
//! This module provides the platform specific integrations. Each supported
//! platform is exposed as a sub-module, exporting the APIs required on each
//! platform.

#[cfg(target_os = "linux")]
pub mod linux_fdo;
