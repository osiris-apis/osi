//! # Build System Operations
//!
//! All operations exposed by the tools of the Osiris Build System are
//! also exposed as Rust functions in this module. This allows performing
//! the same operation from external tools.

use crate::config;

/// ## Emerge Errors
///
/// This is the exhaustive list of possible errors raised by the emerge
/// operation. See each error for details.
pub enum EmergeError {
    /// Platform integration is already present and updating was not
    /// allowed by the caller.
    Already,
    /// Cannot access the specified platform directory.
    PlatformDirectory(std::ffi::OsString),
    /// Creation of the directory at the specified path failed.
    DirectoryCreation(std::ffi::OsString),
    /// Updating the file at the specified path failed with the given error.
    FileUpdate(std::ffi::OsString, std::io::Error),
    /// Removing the file at the specified path failed with the given error.
    FileRemoval(std::ffi::OsString, std::io::Error),
}

/// ## Emerge persistent platform integration
///
/// Write the platform integration for the specified platform to persistent
/// storage. The configuration is sourced for integration parameters. By
/// default, the integration is written to the platform directory for the given
/// platform as specified in the configuration. This base path can be
/// overridden via the `path_override` parameter.
///
/// This function will fail if the platform base directory for the specified
/// platform already exists, unless `update` is `true`. In this case old files
/// are updated to match the new platform integration, and old leftovers are
/// deleted.
pub fn emerge(
    _config: &config::Config,
    _platform: &config::ConfigPlatform,
    _path_override: Option<&std::path::Path>,
    _update: bool,
) -> Result<(), EmergeError> {
    // XXX: To be implemented.
    Ok(())
}
