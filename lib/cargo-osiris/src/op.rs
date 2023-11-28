//! # Build System Operations
//!
//! All operations exposed by the tools of the Osiris Build System are
//! also exposed as Rust functions in this module. This allows performing
//! the same operation from external tools.

use crate::{cargo, config, platform};

/// ## Build Errors
///
/// This is the exhaustive list of possible errors raised by the build
/// operation. See each error for details.
pub enum BuildError {
    /// Cannot create the specified build artifact directory.
    DirectoryCreation(std::ffi::OsString),
    /// Command execution could not commence.
    Exec(String, std::io::Error),
    /// Platform build tools failed.
    Build,
}

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

/// ## Build platform integration
///
/// Perform a full build of the platform integration of the specified platform.
/// If no persistent platform integration is located in the platform directory,
/// an ephemeral platform integration is created and built.
///
/// The target directory of the current crate is used to store any build
/// artifacts. Hence, you likely want to call this through `cargo <external>`
/// to ensure cargo integration is hooked up as expected.
pub fn build(
    config: &config::Config,
    metadata: &cargo::Metadata,
    platform: &config::ConfigPlatform,
) -> Result<(), BuildError> {
    let mut path_build = std::path::PathBuf::new();

    // Create a build directory for all output artifacts of the build process.
    // Re-use the existing directory, if possible, to speed up builds. The
    // directory is created at: `<target>/osi/<platform>`.
    path_build.push(&metadata.target_directory);
    path_build.push("osi");
    path_build.push(&platform.id_symbol);
    std::fs::create_dir_all(path_build.as_path()).map_err(
        |_| BuildError::DirectoryCreation(path_build.as_os_str().to_os_string())
    )?;

    // Invoke the platform-dependent handler. Grant the path-buffers to it, so
    // it can reuse it for further operations.
    match platform.configuration {
        config::ConfigPlatformConfiguration::Android(ref v) => {
            Ok(()) // XXX: To be implemented.
        },
    }
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
