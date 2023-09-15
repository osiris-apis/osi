//! # Persistent Platform Integration
//!
//! The `emerge` operation stores platform integration persistently on disk.
//! Unlike just-in-time integration at build time, this allows adjusting the
//! platform integration to specific needs and retaining modifications across
//! builds.

use crate::config;

/// ## Emerge Errors
///
/// This is the exhaustive list of possible errors raised by the emerge
/// operation. See each error for details.
pub enum Error {
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
) -> Result<(), Error> {
    Ok(())
}
