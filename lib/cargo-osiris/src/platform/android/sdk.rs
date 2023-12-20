//! # Android Platform SDK Access
//!
//! This module provides access to the Android SDK on a target machine. It
//! allows invoking a wide range of functionality of the SDK.

use crate::lib;

/// ## JDK Error
///
/// This is the error-enum of all possible errors raised by the JDK
/// abstraction.
#[derive(Debug)]
pub enum JdkError {
    /// There is no JDK at the given path.
    NoJdk(std::path::PathBuf),
    /// Specified path is not a valid JDK.
    InvalidJdk(std::path::PathBuf),
}

/// ## JDK for Android
///
/// This object represents a JDK suitable for Android on the target machine. It
/// is effectively a wrapper around a path to the JDK root directory.
#[derive(Clone, Debug)]
pub struct Jdk {
    java_home: Option<std::path::PathBuf>,
}

/// ## KDK Error
///
/// This is the error-enum of all possible errors raised by the KDK
/// abstraction.
#[derive(Debug)]
pub enum KdkError {
    /// There is no KDK at the given path.
    NoKdk(std::path::PathBuf),
    /// Specified path is not a valid KDK.
    InvalidKdk(std::path::PathBuf),
}

/// ## KDK for Android
///
/// This object represents a Kotlin Development Kit (KDK) suitable for Android
/// on the target machine. It is effectively a wrapper around a path to the KDK
/// root directory.
#[derive(Clone, Debug)]
pub struct Kdk {
    kotlin_home: Option<std::path::PathBuf>,
    kotlin_compose: Option<std::path::PathBuf>,
}

/// ## SDK Error
///
/// This is the error-enum of all possible errors raised by the Android SDK
/// abstraction.
#[derive(Debug)]
pub enum SdkError {
    /// Uncaught error propagation.
    Uncaught(lib::error::Uncaught),
    /// There is no Android SDK at the given path.
    NoSdk(std::path::PathBuf),
    /// Specified path is not a valid Android SDK.
    InvalidSdk(std::path::PathBuf),
    /// No NDK component is available in the SDK.
    NoNdk,
    /// Specified NDK component is not available or invalid.
    InvalidNdk(std::ffi::OsString),
    /// No Build Tools component is available in the SDK.
    NoBuildTools,
    /// Specified Build Tools component is not available or invalid.
    InvalidBuildTools(std::ffi::OsString),
    /// No platform component available for the given API version.
    NoPlatform(u32),
    /// Invalid platform component for the given API version.
    InvalidPlatform(u32),
}

/// ## Android SDK
///
/// This object represents an Android SDK on the target machine. It is
/// effectively a wrapper around a path to the Android SDK root directory.
#[derive(Clone, Debug)]
pub struct Sdk {
    android_home: std::path::PathBuf,
}

/// ## Android NDK
///
/// This represents a specific instance of the Android SDK NDK component
/// It is effectively a wrapper around a path to an NDK root directory in the
/// Android SDK.
#[derive(Clone, Debug)]
pub struct Ndk {
    path: std::path::PathBuf,
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
        // Convert each entry into its Unicode file name, and drop any non
        // Unicode file names.
        .filter_map(|v| {
            v.map(|v| {
                v.file_name()
                    .into_string()
                    .ok()
            }).transpose()
        })
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
                                lib::str::cmp_natural(
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

impl From<lib::error::Uncaught> for SdkError {
    fn from(v: lib::error::Uncaught) -> Self {
        Self::Uncaught(v)
    }
}

/// ## Create a Class-path
///
/// Concatenate a set of paths into a class-path, suitable for use
/// with JVM and Java tools, or set into the environment.
///
/// Not all paths can be put into a class-path, since there is no way to
/// escape paths. Hence, if any path contains one of the unsupported
/// characters, this will yield `Err` bundled with the unsupported path.
pub fn class_path<T>(
    paths: T,
) -> Result<std::ffi::OsString, std::path::PathBuf>
where
    T: IntoIterator,
    T::Item: AsRef<std::path::Path>,
{
    // Java class-paths use semicolons on Windows (similar to Windows
    // environment variables), but colons everywhere else (similar to POSIX
    // environment variables).
    let (sep_ascii, sep_str) = if cfg!(target_os = "windows") {
        (b';', ";")
    } else {
        (b':', ":")
    };

    let mut acc = std::ffi::OsString::new();
    let mut first = true;

    for i in paths.into_iter() {
        let v = i.as_ref();

        // Class-paths cannot escape the separators, so there is no way
        // to include paths that contain a separator. Reject any such path
        // outright.
        if v.as_os_str()
            .as_encoded_bytes()
            .iter()
            .any(|v| *v == sep_ascii)
        {
            return Err(v.into());
        }

        if first {
            first = false;
        } else {
            acc.push(sep_str);
        }
        acc.push(v);
    }

    Ok(acc)
}

impl Jdk {
    /// ## Create JDK Object from Path
    ///
    /// Create a new JDK Object from a path pointing to the root directory
    /// of the JDK.
    ///
    /// This will perform rudimentory checks on the JDK directory to ensure
    /// it looks valid. This does not guarantee that the JDK is properly
    /// installed, nor does it lock the JDK in any way. It is the
    /// responsibility of the caller to ensure the JDK is accessible and
    /// protected suitably.
    ///
    /// Returns `Err` if the path does not point at a valid directory, or
    /// if the directory does not contain an initialized JDK.
    ///
    /// The `java_home` path is retained verbatim. It is up to the
    /// caller to use an absolute path, if desired.
    ///
    /// If no path is provided, `JAVA_HOME` is expected to be set by the
    /// caller, or the root installation of the JDK is used instead. The
    /// latter assumes no `JAVA_HOME` environment variable is required and all
    /// JDK utilities are accessible from the default environment.
    pub fn new(
        java_home: Option<&std::path::Path>,
    ) -> Result<Self, Box<JdkError>> {
        if let Some(path) = java_home {
            // We expect the JDK to exist and be initialized.
            if !path.is_dir() {
                return Err(Box::new(JdkError::NoJdk(path.to_path_buf())));
            }

            // We have no proper way to identify a JDK, but we simply check for
            // presence of `bin/java` and `bin/javac`, since this is all we need.
            // We also check for `include/jni.h` as sanity test.
            if !path.join("bin/java").is_file()
                || !path.join("bin/javac").is_file()
                || !path.join("include/jni.h").is_file()
            {
                return Err(Box::new(JdkError::InvalidJdk(path.to_path_buf())));
            }
        } else {
            // If no path is provided, `JAVA_HOME` is either inherited, or it
            // is not necessary at all. We simply assume everything is
            // accessible from the default environment, and we defer error
            // detection to the actual JDK accessors.
        }

        // We perform no other checks. It is up to the caller to guarantee
        // that the JDK is usable.
        Ok(Self {
            java_home: java_home.map(|v| v.into()),
        })
    }

    /// ## Yield Command for `javac`
    ///
    /// Yield a new command object for the `javac` command suitable for this
    /// JDK installment.
    pub fn javac(&self) -> std::process::Command {
        if let Some(ref path) = self.java_home {
            let mut cmd = std::process::Command::new(path.join("bin/javac"));

            cmd.env("JAVA_HOME", path);

            cmd
        } else {
            std::process::Command::new("javac")
        }
    }
}

impl Kdk {
    /// ## Create KDK Object from Path
    ///
    /// Create a new KDK Object from a path pointing to the root directory
    /// of the KDK.
    ///
    /// This will perform rudimentory checks on the KDK directory to ensure
    /// it looks valid. This does not guarantee that the KDK is properly
    /// installed, nor does it lock the KDK in any way. It is the
    /// responsibility of the caller to ensure the KDK is accessible and
    /// protected suitably.
    ///
    /// Returns `Err` if the path does not point at a valid directory, or
    /// if the directory does not contain an initialized KDK.
    ///
    /// The `kotlin_home` path is retained verbatim. It is up to the
    /// caller to use an absolute path, if desired.
    ///
    /// If no path is provided, `KOTLIN_HOME` is expected to be set by the
    /// caller, or the root installation of the KDK is used instead. The
    /// latter assumes no `KOTLIN_HOME` environment variable is required and
    /// all KDK utilities are accessible from the default environment.
    pub fn new(
        kotlin_home: Option<&std::path::Path>,
    ) -> Result<Self, Box<KdkError>> {
        let kotlin_compose = if let Some(path) = kotlin_home {
            // We expect the KDK to exist and be initialized.
            if !path.is_dir() {
                return Err(Box::new(KdkError::NoKdk(path.to_path_buf())));
            }

            // We have no proper way to identify a KDK, but we simply check for
            // presence of `bin/kotlinc`, since this is all we need.
            // Additionally, we check for the standard library to ensure the
            // install looks reasonable.
            if !path.join("bin/kotlinc").is_file()
                || !path.join("lib/kotlin-stdlib.jar").is_file()
            {
                return Err(Box::new(KdkError::InvalidKdk(path.to_path_buf())));
            }

            // Check for the Jetpack-Compose compiler plugin. If it is
            // available in the KDK, we remember its path.
            let compose = path.join("lib/kotlin-compose.jar");
            if compose.is_file() {
                Some(compose)
            } else {
                None
            }
        } else {
            // If no path is provided, `KOTLIN_HOME` is either inherited, or it
            // is not necessary at all. We simply assume everything is
            // accessible from the default environment, and we defer error
            // detection to the actual KDK accessors.
            //
            // However, to evaluate whether the jetpack-compose compiler plugin
            // is available, we still have to check for `KOTLIN_HOME`, or the
            // platform default, and see whether the plugin is available.

            let env = std::env::var_os("KOTLIN_HOME");
            let maybe_home: Option<std::path::PathBuf> = match env {
                Some(v) => Some(v.into()),
                None => {
                    if cfg!(target_os = "linux") {
                        Some("/usr/share/kotlin".into())
                    } else {
                        None
                    }
                },
            };

            match maybe_home {
                Some(home) => {
                    let path = home.join("lib/kotlin-compose.jar");
                    if path.is_file() {
                        Some(path)
                    } else {
                        None
                    }
                },
                None => None,
            }
        };

        // We perform no other checks. It is up to the caller to guarantee
        // that the KDK is usable.
        Ok(Self {
            kotlin_home: kotlin_home.map(|v| v.into()),
            kotlin_compose: kotlin_compose,
        })
    }

    /// ## Yield Command for `kotlinc`
    ///
    /// Yield a new command object for the `kotlinc` command suitable for this
    /// KDK installment.
    pub fn kotlinc(&self) -> std::process::Command {
        let mut cmd = match self.kotlin_home {
            Some(ref path) => std::process::Command::new(path.join("bin/kotlinc")),
            None => std::process::Command::new("kotlinc"),
        };

        if let Some(ref path) = self.kotlin_home {
            cmd.env("KOTLIN_HOME", path);
        }

        if let Some(ref path) = self.kotlin_compose {
            let mut arg: std::ffi::OsString = "-Xplugin=".to_string().into();
            arg.push(path);
            cmd.arg(arg);
        }

        cmd
    }
}

impl Sdk {
    /// ## Create SDK Object from Path
    ///
    /// Create a new SDK Object from a path pointing to the root directory
    /// of the Android SDK.
    ///
    /// This will perform rudimentory checks on the SDK directory to ensure
    /// it looks valid. This does not guarantee that the SDK is properly
    /// installed, nor does it lock the SDK in any way. It is the
    /// responsibility of the caller to ensure the SDK is accessible and
    /// protected suitably.
    ///
    /// Returns `Err` if the path does not point at a valid directory, or
    /// if the directory does not contain an initialized Android SDK.
    ///
    /// The `android_home` path is retained verbatim. It is up to the
    /// caller to use an absolute path, if desired.
    pub fn new(android_home: &std::path::Path) -> Result<Self, SdkError> {
        // We expect the SDK to exist and be initialized.
        if !android_home.is_dir() {
            return Err(SdkError::NoSdk(android_home.to_path_buf()));
        }

        // We have no proper way to identify an Android SDK, since all its
        // components are optional. Fortunately, the SDK license must be
        // present if any component is installed, so we use it to identify
        // initialized SDKs.
        if !android_home.join("licenses/android-sdk-license").is_file() {
            return Err(SdkError::InvalidSdk(android_home.to_path_buf()));
        }

        // We perform no other checks. It is up to the caller to guarantee
        // that the SDK is usable.
        Ok(Self {
            android_home: android_home.into(),
        })
    }

    /// ## Query Android Home Directory
    ///
    /// Return the Android Home Directory, which is the path to the root
    /// of the Android SDK.
    ///
    /// The path is a verbatim copy of the path passed to the constructor.
    /// That is, this will be absolute if, and only if, the SDK was
    /// initialized with an absolute path.
    pub fn android_home(&self) -> &std::path::Path {
        self.android_home.as_path()
    }

    /// ## Create NDK Object from SDK
    ///
    /// Create an NDK abstraction for the given Android SDK. Since
    /// multiple NDK versions can be installed in parallel, the
    /// caller must either specify the desired version, or the latest
    /// version is used as default.
    pub fn ndk(
        &self,
        version: Option<&std::ffi::OsStr>,
    ) -> Result<Ndk, SdkError> {
        let mut path = self.android_home().join("ndk");

        if !path.is_dir() {
            return Err(SdkError::NoNdk);
        }

        match version {
            // If no version is specified, we iterate all possible
            // NDKs and pick the latest one.
            None => {
                path.push(
                    dir_latest_entry(path.as_path())
                        .map_err(|v| -> SdkError {
                            lib::error::Uncaught::fold_error(v).into()
                        })?
                        .ok_or_else(|| SdkError::NoNdk)?,
                );
            }

            // If a version is provided, it must be a single non-absolute
            // path component. Hence, `v.parent()` returns
            // `Some(std::path::Path::new(""))`.
            Some(v) => {
                match std::path::Path::new(v).parent() {
                    None => {
                        return Err(SdkError::InvalidNdk(v.into()));
                    }
                    Some(parent) if parent.as_os_str().len() > 0 => {
                        return Err(SdkError::InvalidNdk(v.into()));
                    }
                    _ => {},
                }

                path.push(v);

                if !path.is_dir() {
                    return Err(SdkError::InvalidNdk(v.into()));
                }
            },
        }

        Ok(Ndk {
            path: path,
        })
    }

    /// ## Create Build-Tools Object from SDK
    ///
    /// Create a build-tools abstraction for the given Android SDK. Since
    /// multiple build-tools versions can be installed in parallel, the
    /// caller must either specify the desired version, or the latest
    /// version is used as default.
    pub fn build_tools(
        &self,
        version: Option<&std::ffi::OsStr>,
    ) -> Result<BuildTools, SdkError> {
        let mut path = self.android_home().join("build-tools");

        if !path.is_dir() {
            return Err(SdkError::NoBuildTools);
        }

        match version {
            // If no version is specified, we iterate all possible
            // build-tools and pick the latest one.
            None => {
                path.push(
                    dir_latest_entry(path.as_path())
                        .map_err(|v| -> SdkError {
                            lib::error::Uncaught::fold_error(v).into()
                        })?
                        .ok_or_else(|| SdkError::NoBuildTools)?,
                );
            }

            // If a version is provided, it must be a single non-absolute
            // path component. Hence, `v.parent()` returns
            // `Some(std::path::Path::new(""))`.
            Some(v) => {
                match std::path::Path::new(v).parent() {
                    None => {
                        return Err(SdkError::InvalidBuildTools(v.into()));
                    }
                    Some(parent) if parent.as_os_str().len() > 0 => {
                        return Err(SdkError::InvalidBuildTools(v.into()));
                    }
                    _ => {},
                }

                path.push(v);

                if !path.is_dir() {
                    return Err(SdkError::InvalidBuildTools(v.into()));
                }
            },
        }

        Ok(BuildTools {
            path: path,
        })
    }

    /// ## Acquire Platform Path
    ///
    /// Check the Android SDK for platform files of the given API-level. Return
    /// a path to the base directory of the platform if available. Return an
    /// error no suitable platform component is installed in the Android SDK.
    pub fn platform(
        &self,
        api: u32,
    ) -> Result<std::path::PathBuf, SdkError> {
        let mut path = self.android_home().join("platforms");

        path.push(format!("android-{}", api));
        if !path.is_dir() {
            return Err(SdkError::NoPlatform(api));
        }

        if !path.as_path().join("android.jar").is_file() {
            return Err(SdkError::InvalidPlatform(api));
        }

        Ok(path)
    }
}

impl Ndk {
    /// ## Yield Path to NDK Root Directory
    ///
    /// Yield the path to the root directory of the NDK component.
    pub fn root(&self) -> &std::path::Path {
        self.path.as_path()
    }
}

impl BuildTools {
    /// ## Yield Path to `aapt` Binary
    ///
    /// Yield the path to the `aapt` binary of this build-tools component.
    pub fn aapt(&self) -> std::path::PathBuf {
        self.path.join("aapt")
    }

    /// ## Yield Path to `aapt2` Binary
    ///
    /// Yield the path to the `aapt2` binary of this build-tools component.
    pub fn aapt2(&self) -> std::path::PathBuf {
        self.path.join("aapt2")
    }

    /// ## Yield Path to `apksigner` Binary
    ///
    /// Yield the path to the `apksigner` binary of this build-tools component.
    pub fn apksigner(&self) -> std::path::PathBuf {
        self.path.join("apksigner")
    }

    /// ## Yield Path to `d8` Binary
    ///
    /// Yield the path to the `d8` binary of this build-tools component.
    pub fn d8(&self) -> std::path::PathBuf {
        self.path.join("d8")
    }

    /// ## Yield Path to `zipalign` Binary
    ///
    /// Yield the path to the `zipalign` binary of this build-tools component.
    pub fn zipalign(&self) -> std::path::PathBuf {
        self.path.join("zipalign")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test JDK initialization and verification, as well as basic functionality
    // and sub-object initialization.
    #[test]
    fn jdk_basic() {
        assert!(matches!(
            *Jdk::new(Some(std::path::Path::new("/<invalid>"))).unwrap_err(),
            JdkError::NoJdk(_),
        ));
        assert!(matches!(
            *Jdk::new(Some(std::path::Path::new("/"))).unwrap_err(),
            JdkError::InvalidJdk(_),
        ));
    }

    // Test KDK initialization and verification, as well as basic functionality
    // and sub-object initialization.
    #[test]
    fn kdk_basic() {
        assert!(matches!(
            *Kdk::new(Some(std::path::Path::new("/<invalid>"))).unwrap_err(),
            KdkError::NoKdk(_),
        ));
        assert!(matches!(
            *Kdk::new(Some(std::path::Path::new("/"))).unwrap_err(),
            KdkError::InvalidKdk(_),
        ));
    }

    // Test SDK initialization and verification, as well as basic functionality
    // and sub-object initialization.
    #[test]
    fn sdk_basic() {
        assert!(matches!(
            Sdk::new(std::path::Path::new("/<invalid>")).unwrap_err(),
            SdkError::NoSdk(_),
        ));
        assert!(matches!(
            Sdk::new(std::path::Path::new("/")).unwrap_err(),
            SdkError::InvalidSdk(_),
        ));
    }
}
