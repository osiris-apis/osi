//! # Android Platform SDK Access
//!
//! This module provides access to the Android SDK on a target machine. It
//! allows invoking a wide range of functionality of the SDK.

use crate::util;

/// ## Sdk Error
///
/// This is the error-enum of all possible errors raised by the Android Sdk
/// abstraction.
#[derive(Debug)]
pub enum Error {
    /// Unexpected failure.
    Failure(Box<dyn std::error::Error>),
    /// There is no Android SDK at the given path.
    NoSdk(std::path::PathBuf),
    /// Specified path is not a valid Android Sdk.
    InvalidSdk(std::path::PathBuf),
    /// No Build Tools component is available in the Sdk.
    NoBuildTools,
    /// Specified Build Tools component is not available or invalid.
    InvalidBuildTools(std::ffi::OsString),
}

/// ## Android Sdk
///
/// This object represents an Android Sdk on the target machine. It is
/// effectively a wrapper around a path to the Android Sdk root directory.
#[derive(Clone, Debug)]
pub struct Sdk {
    android_home: std::path::PathBuf,
}

/// ## Android Build Tools
///
/// This represents a specific instance of the Android SDK Build Tools
/// component. It is effectively a wrapper around a path to a build tools
/// root directory in the Android SDK.
#[derive(Clone, Debug)]
pub struct BuildTools {
    path: std::path::PathBuf,
}

// ## Return Directory Entry with Highest Natural Order
//
// Iterate the given directory and return the entry that orders highest of
// all entries. If the directory is empty, then `Ok(None)` is returned.
// If directory enumeration fails with an error, the error is propagated.
//
// Any non-unicode directory entries are ignored, since they cannot be
// compared reasonably.
fn dir_latest_entry(
    dir: &std::path::Path,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    std::fs::read_dir(dir)?
        // Convert each entry into its Unicode file name, or `None`.
        .map(|v| {
            v.map(|v| {
                v.file_name()
                    .into_string()
                    .ok()
            }).transpose()
        })
        // Filter all non-Unicode file names.
        .filter_map(std::convert::identity)
        .reduce(move |acc, v| {
            match (acc, v) {
                // Preserve earliest error, if any.
                (Err(acc), _) => Err(acc),
                // Capture errors as soon as they happen.
                (_, Err(v)) => Err(v),
                // Compare new entry against accumulator.
                (Ok(acc), Ok(v)) => {
                    Ok(
                        core::cmp::max_by(
                            acc,
                            v,
                            |lhs, rhs| {
                                util::str::cmp_natural(
                                    lhs.as_str(),
                                    rhs.as_str(),
                                )
                            },
                        ),
                    )
                },
            }
        })
        .transpose()
        .map_err(|v|v.into())
}

impl Sdk {
    /// ## Create Sdk Object from Path
    ///
    /// Create a new Sdk Object from a path pointing to the root directory
    /// of the Android Sdk.
    ///
    /// This will perform rudimentory checks on the Sdk directory to ensure
    /// it looks valid. This does not guarantee that the Sdk is properly
    /// installed, nor does it lock the Sdk in any way. It is the
    /// responsibility of the caller to ensure the Sdk is accessible and
    /// protected suitably.
    ///
    /// Returns `Err` if the path does not point at a valid directory, or
    /// if the directory does not contain an initialized Android Sdk.
    ///
    /// The `android_home` path is retained verbatim. It is up to the
    /// caller to use an absolute path, if desired.
    pub fn new(android_home: &std::path::Path) -> Result<Self, Box<Error>> {
        // We expect the Sdk to exist and be initialized.
        if !android_home.is_dir() {
            return Err(Box::new(Error::NoSdk(android_home.to_path_buf())));
        }

        // We have no proper way to identify an Android Sdk, since all its
        // components are optional. Fortunately, the Sdk license must be
        // present if any component is installed, so we use it to identify
        // initialized Sdks.
        if !android_home.join("licenses/android-sdk-license").is_file() {
            return Err(Box::new(Error::InvalidSdk(android_home.to_path_buf())));
        }

        // We perform no other checks. It is up to the caller to guarantee
        // that the Sdk is usable.
        Ok(Self {
            android_home: android_home.into(),
        })
    }

    /// ## Query Android Home Directory
    ///
    /// Return the Android Home Directory, which is the path to the root
    /// of the Android Sdk.
    ///
    /// The path is a verbatim copy of the path passed to the constructor.
    /// That is, this will be absolute if, and only if, the Sdk was
    /// initialized with an absolute path.
    pub fn android_home(&self) -> &std::path::Path {
        self.android_home.as_path()
    }

    /// ## Create Build-Tools Object from Sdk
    ///
    /// Create a build-tools abstraction for the given Android SDK. Since
    /// multiple build-tools versions can be installed in parallel, the
    /// caller must either specify the desired version, or the latest
    /// version is used as default.
    pub fn build_tools(
        &self,
        version: Option<&std::ffi::OsStr>,
    ) -> Result<BuildTools, Box<Error>> {
        let mut path = self.android_home().join("build-tools");

        if !path.is_dir() {
            return Err(Box::new(Error::NoBuildTools));
        }

        match version {
            // If no version is specified, we iterate all possible
            // build-tools and pick the latest one.
            None => {
                path.push(
                    dir_latest_entry(path.as_path())
                        .map_err(|v| Box::new(Error::Failure(v)))?
                        .ok_or_else(|| Box::new(Error::NoBuildTools))?
                );
            }

            // If a version is provided, it must be a single non-absolute
            // path component. Hence, `v.parent()` returns
            // `Some(std::path::Path::new(""))`.
            Some(v) => {
                match std::path::Path::new(v).parent() {
                    None => {
                        return Err(Box::new(Error::InvalidBuildTools(v.into())));
                    }
                    Some(parent) if parent.as_os_str().len() > 0 => {
                        return Err(Box::new(Error::InvalidBuildTools(v.into())));
                    }
                    _ => {},
                }

                path.push(v);

                if !path.is_dir() {
                    return Err(Box::new(Error::InvalidBuildTools(v.into())));
                }
            },
        }

        Ok(BuildTools {
            path: path,
        })
    }
}

impl BuildTools {
    /// ## Yield Path to `aapt2` Binary
    ///
    /// Yield the path to the `aapt2` binary of this build-tools component.
    pub fn aapt2(&self) -> std::path::PathBuf {
        self.path.join("aapt2")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test Sdk initialization and verification, as well as basic functionality
    // and sub-object initialization.
    #[test]
    fn sdk_basic() {
        assert!(matches!(
            *Sdk::new(std::path::Path::new("/<invalid>")).unwrap_err(),
            Error::NoSdk(_),
        ));
        assert!(matches!(
            *Sdk::new(std::path::Path::new("/")).unwrap_err(),
            Error::InvalidSdk(_),
        ));
    }
}
