//! # Build System Operations
//!
//! All operations exposed by the tools of the Osiris Build System are
//! also exposed as Rust functions in this module. This allows performing
//! the same operation from external tools.

use crate::{cargo, config, lib, platform};

/// ## Build Errors
///
/// This is the exhaustive list of possible errors raised by the build
/// operation. See each error for details.
///
/// XXX: File-system errors should carry the IO-Error.
pub enum BuildError {
    /// Uncaught error propagation.
    Uncaught(lib::error::Uncaught),
    /// Cannot traverse the specified directory.
    DirectoryTraversal(std::ffi::OsString),
    /// Cannot create the specified build artifact directory.
    DirectoryCreation(std::ffi::OsString),
    /// Cannot remove the specified build artifact directory.
    DirectoryRemoval(std::ffi::OsString),
    /// Updating the file at the specified path failed with the given error.
    FileUpdate(std::path::PathBuf, std::io::Error),
    /// Copying a file failed with the given error.
    FileCopy(std::path::PathBuf, std::path::PathBuf, std::io::Error),
    /// Execution of the given tool could not commence.
    Exec(String, std::io::Error),
    /// Given tool failed executing.
    Exit(String, std::process::ExitStatus),
    /// Cargo specific errors.
    Cargo(cargo::Error),
    /// Android platform specific errors.
    AndroidPlatform(platform::android::BuildError),
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

impl From<lib::error::Uncaught> for BuildError {
    fn from(v: lib::error::Uncaught) -> Self {
        Self::Uncaught(v)
    }
}

impl From<cargo::Error> for BuildError {
    fn from(v: cargo::Error) -> Self {
        Self::Cargo(v)
    }
}

impl From<platform::android::BuildError> for BuildError {
    fn from(v: platform::android::BuildError) -> Self {
        Self::AndroidPlatform(v)
    }
}

/// ## Enumerate Directory Recursively
///
/// Recursively walk a directory and collect all entries, except for
/// directories that are followed. This will follow symlinks and must thus be
/// used carefully.
pub fn lsrdir(path: &std::path::Path) -> Result<Vec<std::path::PathBuf>, BuildError> {
    let mut past = std::collections::BTreeSet::new();
    let mut todo: Vec<std::path::PathBuf> = vec![path.into()];
    let mut res = Vec::new();

    while let Some(ref dir) = todo.pop() {
        let entries = std::fs::read_dir(dir).map_err(
            |_| BuildError::DirectoryTraversal(dir.into()),
        )?;
        for iter in entries {
            let entry = iter.map_err(
                |_| BuildError::DirectoryTraversal(dir.into()),
            )?;
            let mut entry_ft = entry.file_type().map_err(
                |_| BuildError::DirectoryTraversal(dir.into()),
            )?;
            let entry_path = dir.join(entry.file_name());

            // Remember all paths we visited. Skip any path if we visit it
            // twice, to avoid entering a loop. Note that if circular symlinks
            // are used, they will eventually hit system limits on nesting
            // depth or path length.
            if !past.insert(entry_path.clone()) {
                continue;
            }

            if entry_ft.is_symlink() {
                let entry_md = std::fs::metadata(entry_path.as_path()).map_err(
                    |_| BuildError::DirectoryTraversal((&entry_path).into()),
                )?;

                entry_ft = entry_md.file_type();
                if entry_ft.is_symlink() {
                    return Err(BuildError::DirectoryTraversal((&entry_path).into()));
                }
            }

            if entry_ft.is_dir() {
                todo.push(entry_path);
            } else {
                res.push(entry_path);
            }
        }
    }

    Ok(res)
}

/// ## Create Build Directory
///
/// This is a wrapper around `std::fs::create_dir_all()` that properly
/// converts failures into the local error domain.
pub fn mkdir(path: &std::path::Path) -> Result<(), BuildError> {
    std::fs::create_dir_all(path).map_err(
        |_| BuildError::DirectoryCreation(path.into()),
    )?;

    Ok(())
}

/// ## Remove Build Directory
///
/// This is a wrapper around `std::fs::remove_dir_all()` that properly
/// converts failures into the local error domain.
///
/// This this a no-op if the target path does not exist in the file
/// system. Note that this is only checked once, and thus may still fail
/// when another removal runs in parallel.
pub fn rmdir(path: &std::path::Path) -> Result<(), BuildError> {
    if path.exists() {
        std::fs::remove_dir_all(path).map_err(
            |_| BuildError::DirectoryRemoval(path.into()),
        )?;
    }

    Ok(())
}

/// ## Copy a file
///
/// This is a wrapper around `std::fs::copy()` that converts errors into
/// the local error domain.
pub fn copy_file(
    src: &std::path::Path,
    dst: &std::path::Path,
) -> Result<(), BuildError> {
    std::fs::copy(src, dst).map_err(
        |v| BuildError::FileCopy(src.into(), dst.into(), v),
    )?;

    Ok(())
}

/// ## Update a file if required
///
/// This writes the given content to the specified file, but only if the file
/// content does not already match the new content. This avoids modifying a
/// file unless necessary. Thus, the file timestamp is only modified if the
/// content really changed.
///
/// Note that this reads in the entire file content. Thus, use it only on
/// trusted and small'ish content.
///
/// Returns `true` if the file has new content and thus changed. Returns
/// `false` if the file was not modified.
pub fn update_file(
    path: &std::path::Path,
    content: &str,
) -> Result<bool, BuildError> {
    // If the desired content is an empty file, we have to know whether the
    // file existed before we open it. Otherwise, we might create it when
    // opening it, and then cannot tell if we actually did.
    let maybe_new = content.is_empty() && !path.is_file();

    // Open the file read+write and create it if it does not exist, yet.
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .map_err(
            |v| BuildError::FileUpdate(path.into(), v),
        )?;

    // Read the entire file content into memory.
    let mut old = String::new();
    <std::fs::File as std::io::Read>::read_to_string(&mut f, &mut old)
        .map_err(
            |v| BuildError::FileUpdate(path.into(), v),
        )?;

    // If the file has to be updated, rewind the position, truncate the file
    // and write the new contents.
    let new = if old != content {
        <std::fs::File as std::io::Seek>::rewind(&mut f)
            .map_err(
                |v| BuildError::FileUpdate(path.into(), v),
            )?;

        f.set_len(0).map_err(
            |v| BuildError::FileUpdate(path.into(), v),
        )?;

        <std::fs::File as std::io::Write>::write_all(&mut f, content.as_bytes())
            .map_err(
                |v| BuildError::FileUpdate(path.into(), v),
            )?;

        f.sync_all().map_err(
            |v| BuildError::FileUpdate(path.into(), v),
        )?;

        true
    } else {
        // The file matches the desired content. Hence, it is only new if the
        // file was created with empty content when opening it.
        maybe_new
    };

    Ok(new)
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
            platform::android::build(
                config,
                metadata,
                platform,
                v,
                path_build.as_path(),
            )
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
