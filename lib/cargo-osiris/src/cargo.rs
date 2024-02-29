//! # Cargo Interaction
//!
//! Provide Cargo sub-command executors and parsers. These allow invoking Cargo
//! subcommands programmatically, parsing the output into machine-readable
//! types.

use crate::misc;
use std::collections::{BTreeMap, BTreeSet};

/// Error definitions for Osiris Metadata parsing.
#[derive(Debug)]
pub enum MdOsiError {
    /// Invalid type for the specified field
    TypeInvalid(&'static str, &'static str),
    /// Supported range of the selected type was exceeded
    TypeExceeded(&'static str),
    /// Mandatory key is missing
    KeyMissing(&'static str),
    /// Key cannot be specified with conflicting alternatives
    KeyExclusive(&'static str),
    /// Specified version is higher/lower than supported by this implementation.
    VersionUnsupported(u32),
}

/// Error definitions for all possible errors of the Cargo metadata extraction.
#[derive(Debug)]
pub enum Error {
    /// Execution of `cargo` could not commence
    Exec(std::io::Error),
    /// `cargo` exited without success
    Cargo(std::process::ExitStatus),
    /// Unicode decoding error
    Unicode(std::str::Utf8Error),
    /// JSON decoding error
    Json,
    /// No package specified, nor does the Cargo workspace have a root
    NoPackage,
    /// Unknown package reference
    UnknownPackage(String),
    /// Ambiguous package reference
    AmbiguousPackage(String),
    /// Data decoding error
    Data,
    /// Osiris Metadata parsing errors
    MdOsi(MdOsiError),
}

/// Cargo arguments shared across different Cargo sub-commands. They select
/// the workspace and package to operate on, as well as the configuration for
/// the package.
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Arguments {
    pub default_features: Option<bool>,
    pub features: Vec<String>,
    pub frozen: Option<bool>,
    pub manifest_path: Option<std::path::PathBuf>,
    pub package: Option<String>,
    pub profile: Option<String>,
    pub target_dir: Option<std::path::PathBuf>,
}

/// Metadata about the application independent of the target platform.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MdOsiApplication {
    /// Identifier of the application. Used to register and identify the
    /// application. Must not change over the life of the application. Only
    /// alphanumeric and `-`, `_` allowed. Non-ASCII allowed but might break
    /// external tools.
    pub id: Option<String>,
    /// Human-readable name of the application.
    pub name: Option<String>,
}

/// Metadata about the application and library for the Android platform.
/// These are one-to-one mappings of their respective counterparts in the
/// Android SDK.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MdOsiPlatformAndroid {
    pub application_id: Option<String>,
    pub namespace: Option<String>,

    pub compile_sdk: Option<u32>,
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,

    pub abis: Option<Vec<String>>,

    pub version_code: Option<u32>,
    pub version_name: Option<String>,
}

/// Metadata about the application and framework for the macOS platform.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MdOsiPlatformMacos {
}

/// Metadata specific to a platform, indexed by the name of the platform.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MdOsiPlatformConfiguration {
    /// Android platform table
    Android(MdOsiPlatformAndroid),
    /// Macos platform table
    Macos(MdOsiPlatformMacos),
}

/// Metadata about a platform integration supported by the application.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MdOsiPlatform {
    /// Custom ID of the platform integration.
    pub id: String,
    /// Path to the platform integration root relative from the configuration.
    pub path: Option<String>,

    /// Platform specific configuration.
    pub configuration: Option<MdOsiPlatformConfiguration>,
}

/// Version `1` of the Osiris metadata format.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MdOsiV1 {
    /// Application table specifying properties of the application
    /// itself.
    pub application: Option<MdOsiApplication>,
    /// Platform table specifying all properties of the platform
    /// integration for all supported platforms.
    pub platforms: Vec<MdOsiPlatform>,
}

/// Osiris metadata that was embedded as `package.metadata.osiris` in a Cargo
/// manifest.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MdOsi {
    /// Version `1` of the metadata format.
    V1(MdOsiV1),
}

/// Metadata required to bundle an application of library for the Android
/// platform.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MetadataAndroid {
    /// Java source directories to be included in an Android build
    pub java_dirs: Vec<std::path::PathBuf>,
    /// Kotlin source directories to be included in an Android build
    pub kotlin_dirs: Vec<std::path::PathBuf>,
    /// Android manifest to be included in an Android build
    pub manifest_file: Option<std::path::PathBuf>,
    /// Android resource directories to be included in an Android build
    pub resource_dirs: Vec<std::path::PathBuf>,
}

/// Reduced metadata as returned by an invocation of `cargo-metadata`. Only the
/// pieces needed by this implementation are retained.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Metadata {
    /// Sets of Android-related build metadata
    pub android_sets: Vec<MetadataAndroid>,
    /// Osiris package metadata
    pub osiris: Option<MdOsi>,
    /// Package ID of the target package
    pub package_id: String,
    /// Target directory of the package build
    pub target_directory: String,
}

// Intermediate state after cargo-metadata returned, but the blob was not yet
// parsed into the metadata object.
#[derive(Debug)]
struct MetadataBlob {
    pub json: serde_json::Value,
}

/// Open-coded structure with all parameters for a query to `cargo-metadata`.
/// It is to be filled in by the caller.
#[derive(Clone, Debug)]
pub struct MetadataQuery<'ctx> {
    /// Package, workspace, and configuration arguments for Cargo.
    pub cargo_arguments: &'ctx Arguments,
    /// The target platform to compile for. If `None`, a generic request for
    /// all possible targets is performed.
    pub target: Option<String>,
}

/// Output of a `cargo build` run with only relevant pieces retained.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Build {
    /// List of absolute paths to artifacts produced by the build. This is a
    /// filtered list including only final production artifacts.
    pub artifacts: Vec<String>,
}

// Intermediate state after cargo-build returned, but the blob was not yet
// parsed into the `Build` object.
#[derive(Debug)]
struct BuildBlob {
    pub json: Vec<serde_json::Value>,
}

/// Parameters to a `cargo build` operation. To be filled in by the query
/// requester.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BuildQuery<'ctx> {
    /// Package, workspace, and configuration arguments for Cargo.
    pub cargo_arguments: &'ctx Arguments,
    /// Environment variables to set for the build.
    pub envs: Vec<(std::ffi::OsString, std::ffi::OsString)>,
    /// The target platform to compile for.
    pub target: Option<String>,
}

// Return the Cargo command to use for invocations of Cargo. This will
// look at the `CARGO` environment variable first, and if unset use the
// default `cargo` command.
//
// Note that Cargo sub-commands get the `CARGO` environment variable set
// unconditionally, and thus ensure that the correct toolchain is used.
fn cargo_command() -> std::ffi::OsString {
    std::env::var_os("CARGO").unwrap_or("cargo".into())
}

impl core::fmt::Display for MdOsiError {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        match self {
            MdOsiError::TypeInvalid(v, t) => fmt.write_fmt(core::format_args!("Specified entry requires a value of a different type: `{}` requires type `{}`", v, t)),
            MdOsiError::TypeExceeded(v) => fmt.write_fmt(core::format_args!("Specified entry exceeded the maximum supported range for its type: {}", v)),
            MdOsiError::KeyMissing(v) => fmt.write_fmt(core::format_args!("Required entry was not specified: {}", v)),
            MdOsiError::KeyExclusive(v) => fmt.write_fmt(core::format_args!("Exclusive entry was specified with conflicts: {}", v)),
            MdOsiError::VersionUnsupported(v) => fmt.write_fmt(core::format_args!("Specified version is not supported: {}", v)),
        }
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        match self {
            Error::Exec(e) => fmt.write_fmt(core::format_args!("Execution of `cargo` could not commence (io-error: {})", e)),
            Error::Cargo(e) => fmt.write_fmt(core::format_args!("`cargo` failed unexpectedly (exit-code: {})", e)),
            Error::Unicode(e) => fmt.write_fmt(core::format_args!("`cargo` returned invalid Unicode data (utf8-error: {})", e)),
            Error::Json => fmt.write_fmt(core::format_args!("`cargo` returned invalid JSON data")),
            Error::NoPackage => fmt.write_fmt(core::format_args!("No package specified, nor does the Cargo workspace have a root package")),
            Error::UnknownPackage(v) => fmt.write_fmt(core::format_args!("Cannot resolve requested package name: {}", v)),
            Error::AmbiguousPackage(v) => fmt.write_fmt(core::format_args!("Ambiguous package name: {}", v)),
            Error::Data => fmt.write_fmt(core::format_args!("Cannot decode Cargo metadata")),
            Error::MdOsi(e) => fmt.write_fmt(core::format_args!("Cannot parse Osiris metadata: {}", e)),
        }
    }
}

impl core::convert::From<MdOsiError> for Error {
    fn from(v: MdOsiError) -> Self {
        Error::MdOsi(v)
    }
}

impl Arguments {
    /// Yield whether frozen operation should be chosen.
    pub fn frozen(&self) -> bool {
        self.frozen.unwrap_or(false)
    }

    /// Yield the path to the manifest, returning the default if none was
    /// specified.
    pub fn manifest_path(&self) -> &std::path::Path {
        self.manifest_path.as_deref().unwrap_or(
            std::path::Path::new("./Cargo.toml"),
        )
    }

    /// Yield whether default features should be disabled with this
    /// configuration.
    pub fn no_default_features(&self) -> bool {
        !self.default_features.unwrap_or(true)
    }
}

impl MetadataBlob {
    fn from_str(data: &str) -> Result<Self, Error> {
        Ok(Self {
            json: serde_json::from_str(data).map_err(|_| Error::Json)?,
        })
    }

    fn from_bytes(data: &[u8]) -> Result<Self, Error> {
        Self::from_str(
            std::str::from_utf8(data).map_err(|v| Error::Unicode(v))?,
        )
    }

    // Resolve a key to the package ID of a local package. This traverses the
    // `packages` array in the Cargo metadata blob to find a package with the
    // given name and then returns its corresponding ID. Any non-local packages
    // are completely ignored and do not affect this resolution.
    //
    // If there are less than, or more than, 1 package with the given name, an
    // error is returned due to ambiguous package IDs.
    //
    // If the key matches a local package ID (rather than name), it will always
    // resolve to that ID. This resolves any ambiguity.
    fn resolve_local_package(&self, key: &str) -> Result<String, Error> {
        let mut candidates = BTreeSet::new();

        let packages = match self.json.get("packages") {
            Some(serde_json::Value::Array(v)) => v,
            _ => return Err(Error::UnknownPackage(key.to_string())),
        };

        for pkg in packages.iter() {
            let id = match pkg.get("id") {
                Some(serde_json::Value::String(v)) => v,
                _ => continue,
            };
            let name = match pkg.get("name") {
                Some(serde_json::Value::String(v)) => Some(v.as_str()),
                _ => None,
            };

            // Local packages have `source` unset or set to `null`.
            if let None | Some(serde_json::Value::Null) = pkg.get("source") {
                // If the key matches any package ID directly, it is already
                // fully specified and we can just return it.
                if key == id {
                    return Ok(id.clone());
                }

                // Otherwise, if the key matches the package name, keep the
                // package ID as candidate.
                if Some(key) == name {
                    candidates.insert(id);
                }
            }
        }

        match candidates.len() {
            0 => Err(Error::UnknownPackage(key.to_string())),
            1 => Ok((*candidates.first().unwrap()).clone()),
            _ => Err(Error::AmbiguousPackage(key.to_string())),
        }
    }

    // Take a Cargo Metadata blob and collect the package IDs of all packages
    // that are involved in a normal compilation. The caller can specify which
    // packages are compiled.
    //
    // The `resolve.nodes` array usually contains exactly this set. However,
    // it also contains build and dev dependencies, as well as dependencies
    // of workspace packages other than the requested package. Hence, this
    // function collects exactly the required package IDs.
    fn involved_ids(
        &self,
        start: &str,
    ) -> BTreeSet<String> {
        let mut roots = BTreeSet::<&str>::new();
        let mut ids = BTreeSet::<String>::new();
        let mut todo = BTreeSet::<&str>::new();
        let mut depmap = BTreeMap::<&str, BTreeSet<&str>>::new();

        // Fetch the objects in the resolved dependency map of the
        // Cargo metadata blob. If no resolve-map is present, or if no
        // nodes are listed, there is nothing to resolve and we return
        // an empty set.
        let resolve = match self.json.get("resolve") {
            Some(serde_json::Value::Object(v)) => v,
            _ => return ids,
        };
        let nodes = match resolve.get("nodes") {
            Some(serde_json::Value::Array(v)) => v,
            _ => return ids,
        };

        // Use the specified start node as root for the resolution. The code
        // supports multiple roots, yet we currently only pass in a single one.
        roots.insert(start);

        // Iterate all nodes and collect their actual code-dependencies, but
        // ignore any build or dev dependencies. Push each node into the
        // dependency map with their dependencies as a set.
        for node in nodes.iter() {
            let id = match node.get("id") {
                Some(serde_json::Value::String(v)) => v,
                _ => continue
            };
            let deps = match node.get("deps") {
                Some(serde_json::Value::Array(v)) => v,
                _ => continue,
            };

            // Accumulate all dependencies of `id` in a set, so we can
            // later on push them into the dependency map.
            let mut acc = BTreeSet::<&str>::new();
            for dep in deps.iter() {
                let pkg = match dep.get("pkg") {
                    Some(serde_json::Value::String(v)) => v,
                    _ => continue
                };
                let dep_kinds = match dep.get("dep_kinds") {
                    Some(serde_json::Value::Array(v)) => v,
                    _ => continue
                };

                // A dependency can be of multiple kinds. Only a kind of `Null`
                // denotes actual code dependencies. See if it is there.
                for dep_kind in dep_kinds.iter() {
                    if let Some(serde_json::Value::Null) = dep_kind.get("kind") {
                        acc.insert(pkg.as_str());
                        break;
                    }
                }
            }

            depmap.insert(id.as_str(), acc);
        }

        // For every root node, start at the root package and collect all its
        // dependencies into the final set. Repeat this for each dependency,
        // avoiding cycles. Yield the final set to the caller.
        for root in roots {
            todo.clear();
            todo.insert(root);
            ids.insert(root.to_string());
            while let Some(next) = todo.pop_first() {
                if let Some(deps) = depmap.get(next) {
                    for &dep in deps.iter() {
                        if !ids.contains(dep) {
                            todo.insert(dep);
                            ids.insert(dep.to_string());
                        }
                    }
                }
            }
        }

        ids
    }

    fn get_str(
        map: &serde_json::value::Map<String, serde_json::Value>,
        key: &'static str,
        path: &'static str,
    ) -> Result<Option<String>, Error> {
        match map.get(key) {
            None => Ok(None),
            Some(serde_json::Value::String(v)) => {
                Ok(Some(v.clone()))
            },
            Some(_) => {
                Err(MdOsiError::TypeInvalid(path, "string"))
            },
        }.map_err(|e| e.into())
    }

    fn get_u32(
        map: &serde_json::value::Map<String, serde_json::Value>,
        key: &'static str,
        path: &'static str,
    ) -> Result<Option<u32>, Error> {
        match map.get(key) {
            None => Ok(None),
            Some(serde_json::Value::Number(v)) => {
                if let Some(v) = v.as_u64() {
                    if let Ok(v) = u32::try_from(v) {
                        Ok(Some(v))
                    } else {
                        Err(MdOsiError::TypeExceeded(path))
                    }
                } else {
                    Err(MdOsiError::TypeExceeded(path))
                }
            },
            Some(_) => {
                Err(MdOsiError::TypeInvalid(path, "number"))
            },
        }.map_err(|e| e.into())
    }

    // Helper for `parse_mdosi()` that extracts the Android platform configuration.
    fn parse_mdosi_android(
        &self,
        android: &serde_json::value::Map<String, serde_json::Value>,
    ) -> Result<MdOsiPlatformAndroid, Error> {
        let v_application_id = Self::get_str(android, "application-id", "platforms.[].android.application-id")?;
        let v_namespace = Self::get_str(android, "namespace", "platforms.[].android.namespace")?;
        let v_compile_sdk = Self::get_u32(android, "compile-sdk", "platforms.[].android.compile-sdk")?;
        let v_min_sdk = Self::get_u32(android, "min-sdk", "platforms.[].android.min-sdk")?;
        let v_target_sdk = Self::get_u32(android, "target-sdk", "platforms.[].android.target-sdk")?;
        let v_version_code = Self::get_u32(android, "version-code", "platforms.[].android.version-code")?;
        let v_version_name = Self::get_str(android, "version-name", "platforms.[].android.version-name")?;

        let v_abis = match android.get("abis") {
            None => None,
            Some(serde_json::Value::Array(abis)) => {
                let mut acc = Vec::new();
                for abi in abis.iter() {
                    match abi {
                        serde_json::Value::String(v) => {
                            acc.push(v.clone());
                        },
                        _ => {
                            return Err(MdOsiError::TypeInvalid("platforms.[].android.abis.[]", "string").into())
                        }
                    }
                }
                Some(acc)
            },
            Some(_) => {
                return Err(MdOsiError::TypeInvalid("platforms.[].android.abis", "array").into());
            },
        };

        Ok(MdOsiPlatformAndroid {
            application_id: v_application_id,
            namespace: v_namespace,

            compile_sdk: v_compile_sdk,
            min_sdk: v_min_sdk,
            target_sdk: v_target_sdk,

            abis: v_abis,

            version_code: v_version_code,
            version_name: v_version_name,
        })

    }

    // Helper for `parse_mdosi()` that extracts the macOS platform configuration.
    fn parse_mdosi_macos(
        &self,
        _macos: &serde_json::value::Map<String, serde_json::Value>,
    ) -> Result<MdOsiPlatformMacos, Error> {
        Ok(MdOsiPlatformMacos {
        })
    }

    // Helper for `parse()` that extracts the Osiris metadata from the package
    // metadata.
    fn parse_mdosi(
        &self,
        pkgmd: &serde_json::value::Map<String, serde_json::Value>,
    ) -> Result<Option<MdOsi>, Error> {
        // Get the top-level entry for Osiris metadata.
        let json_osiris = match pkgmd.get("osiris") {
            None => return Ok(None),
            Some(serde_json::Value::Object(v)) => Ok(v),
            Some(_) => Err(MdOsiError::TypeInvalid("osiris", "object")),
        }?;

        // Figure out the metadata version.
        let _version = match Self::get_u32(json_osiris, "version", "version")? {
            None => Err(MdOsiError::KeyMissing("version")),
            Some(1) => Ok(1),
            Some(v) => Err(MdOsiError::VersionUnsupported(v)),
        }?;

        // Create the top-level object and parse everything
        // into it. Only version 1 is defined so far.
        let mut mdosi = MdOsiV1 {
            application: None,
            platforms: Vec::new(),
        };

        // Extract the `application` data.
        match json_osiris.get("application") {
            None => {},
            Some(serde_json::Value::Object(application)) => {
                let mut mdosi_app = MdOsiApplication {
                    id: None,
                    name: None,
                };

                match application.get("id") {
                    None => {},
                    Some(serde_json::Value::String(id_str)) => {
                        mdosi_app.id = Some(id_str.clone());
                    },
                    Some(_) => {
                        return Err(MdOsiError::TypeInvalid("application.id", "string").into());
                    },
                }
                match application.get("name") {
                    None => {},
                    Some(serde_json::Value::String(name_str)) => {
                        mdosi_app.name = Some(name_str.clone());
                    },
                    Some(_) => {
                        return Err(MdOsiError::TypeInvalid("application.name", "string").into());
                    },
                }

                mdosi.application = Some(mdosi_app);
            },
            Some(_) => {
                return Err(MdOsiError::TypeInvalid("application", "object").into());
            }
        }

        // Extract the `platforms` data.
        match json_osiris.get("platforms") {
            None => {},
            Some(serde_json::Value::Array(platforms)) => {
                for platform in platforms.iter() {
                    let id = match platform.get("id") {
                        None => {
                            return Err(MdOsiError::KeyMissing("platforms.[].id").into());
                        },
                        Some(serde_json::Value::String(id_str)) => {
                            id_str.clone()
                        },
                        Some(_) => {
                            return Err(MdOsiError::TypeInvalid("platforms.[].id", "string").into());
                        },
                    };

                    let mut mdosi_pf = MdOsiPlatform {
                        id: id,
                        path: None,
                        configuration: None,
                    };

                    match platform.get("path") {
                        None => {},
                        Some(serde_json::Value::String(path_str)) => {
                            mdosi_pf.path = Some(path_str.clone());
                        },
                        Some(_) => {
                            return Err(MdOsiError::TypeInvalid("platforms.[].path", "string").into());
                        },
                    }

                    mdosi_pf.configuration = match (
                        platform.get("android"),
                        platform.get("macos"),
                    ) {
                        (None, None) => Ok(None),
                        (Some(_), Some(_)) => {
                            Err(MdOsiError::KeyExclusive("platforms.[].{android,macos}").into())
                        },
                        (Some(serde_json::Value::Object(android)), None) => {
                            self.parse_mdosi_android(android).map(|v| Some(MdOsiPlatformConfiguration::Android(v)))
                        },
                        (Some(_), None) => {
                            Err(MdOsiError::TypeInvalid("platforms.<platform>.android", "object").into())
                        },
                        (None, Some(serde_json::Value::Object(macos))) => {
                            self.parse_mdosi_macos(macos).map(|v| Some(MdOsiPlatformConfiguration::Macos(v)))
                        },
                        (None, Some(_)) => {
                            Err(MdOsiError::TypeInvalid("platforms.<platform>.macos", "object").into())
                        },
                    }?;

                    mdosi.platforms.push(mdosi_pf);
                }
            },
            Some(_) => {
                return Err(MdOsiError::TypeInvalid("platforms", "array").into());
            },
        }

        Ok(Some(MdOsi::V1(mdosi)))
    }

    // Parse all desired fields in the manifest blob and expose them as a
    // new Metadata object.
    fn parse(&self, query: &MetadataQuery) -> Result<Metadata, Error> {
        // Extract `target_directory` from the blob. It is a mandatory field.
        let v_target_directory = self.json.get("target_directory").ok_or(Error::Data)?
            .as_str().ok_or(Error::Data)?
            .to_string();

        // If no package name was specified, find the root package in the metadata. If none
        // is present, raise an error to the caller.
        let root_raw = match &query.cargo_arguments.package {
            None => match self.json.get("resolve") {
                Some(serde_json::Value::Object(v)) => match v.get("root") {
                    Some(serde_json::Value::String(v)) => Ok(v),
                    _ => Err(Error::NoPackage),
                },
                _ => Err(Error::NoPackage),
            },
            Some(v) => Ok(v),
        }?;
        let root = self.resolve_local_package(root_raw)?;

        // Walk the dependency graph and collect all packages that are part of
        // this compilation. We have to do this, since only the dependency
        // graph is affected by target-filtering and feature-selection, and we
        // want to avoid any build or dev dependencies.
        let ids = self.involved_ids(&root);

        // Now walk the package list and extract all data we desire.
        let mut android_sets = Vec::new();
        let mut pkgmd_osi = None;
        if let Some(serde_json::Value::Array(packages)) = self.json.get("packages") {
            for pkg in packages.iter() {
                let mut java_dirs = Vec::new();
                let mut kotlin_dirs = Vec::new();
                let mut manifest_file = None;
                let mut res_dirs = Vec::new();

                // Skip any packages that we are not interested in.
                let id = match pkg.get("id") {
                    Some(serde_json::Value::String(v)) => v,
                    _ => continue
                };
                if !ids.contains(id) {
                    continue;
                }
                let is_root = *id == root;

                // Get the absolute path to the package root. We need this
                // normalize other paths in the package metadata.
                let manifest_path = match pkg.get("manifest_path") {
                    Some(serde_json::Value::String(v)) => v,
                    _ => continue
                };
                let package_path = misc::absdir(&manifest_path);

                // Extract all metadata we desire.
                if let Some(serde_json::Value::Object(metadata)) = pkg.get("metadata") {
                    // The `android` namespace declares metadata required to
                    // compile a crate for Android platforms. This includes
                    // information on bundled JVM sources, as well as Android
                    // resource files.
                    //
                    // XXX: We should reject paths that are absolute or point
                    //      outside the package.
                    if let Some(serde_json::Value::Object(android)) = metadata.get("android") {
                        if let Some(serde_json::Value::Object(java)) = android.get("java") {
                            if let Some(serde_json::Value::Array(dirs)) = java.get("source-dirs") {
                                for dir in dirs.iter() {
                                    if let serde_json::Value::String(dir_str) = dir {
                                        java_dirs.push(package_path.as_path().join(dir_str));
                                    }
                                }
                            }
                        }
                        if let Some(serde_json::Value::Object(kotlin)) = android.get("kotlin") {
                            if let Some(serde_json::Value::Array(dirs)) = kotlin.get("source-dirs") {
                                for dir in dirs.iter() {
                                    if let serde_json::Value::String(dir_str) = dir {
                                        kotlin_dirs.push(package_path.as_path().join(dir_str));
                                    }
                                }
                            }
                        }
                        if let Some(serde_json::Value::String(manifest_str)) = android.get("manifest-file") {
                            manifest_file = Some(manifest_str);
                        }
                        if let Some(serde_json::Value::Array(dirs)) = android.get("resource-dirs") {
                            for dir in dirs.iter() {
                                if let serde_json::Value::String(dir_str) = dir {
                                    res_dirs.push(package_path.as_path().join(dir_str));
                                }
                            }
                        }
                    }

                    if is_root {
                        pkgmd_osi = self.parse_mdosi(metadata)?;
                    }
                }

                // Store the metadata if any value is set.
                if java_dirs.len() > 0
                    || kotlin_dirs.len() > 0
                    || manifest_file.is_some()
                    || res_dirs.len() > 0
                {
                    android_sets.push(MetadataAndroid {
                        java_dirs: java_dirs,
                        kotlin_dirs: kotlin_dirs,
                        manifest_file: manifest_file.map(
                            |v| std::path::Path::new(v).into(),
                        ),
                        resource_dirs: res_dirs,
                    });
                }
            }
        }

        // Return the parsed `Metadata` object.
        Ok(
            Metadata {
                android_sets: android_sets,
                osiris: pkgmd_osi,
                package_id: root,
                target_directory: v_target_directory,
            }
        )
    }
}

impl<'ctx> MetadataQuery<'ctx> {
    /// Query Cargo for all metadata for the specified workspace. This will
    /// invoke `cargo metadata` and parse all the cargo metadata into the
    /// `Metadata` object. Only the bits required by the crate are fetched,
    /// everything else is ignored.
    pub fn run(&self) -> Result<Metadata, Error> {
        // Build the cargo-metadata invocation.
        let mut cmd = std::process::Command::new(cargo_command());
        cmd.args([
            "metadata",
            "--format-version=1",
            "--offline",
            "--quiet",
        ]);

        // Append the selected features.
        for v in &self.cargo_arguments.features {
            cmd.arg("--features");
            cmd.arg(v);
        }

        // Freeze dependencies, if requested.
        if self.cargo_arguments.frozen() {
            cmd.arg("--frozen");
        }

        // Append path to the manifest.
        cmd.arg("--manifest-path");
        cmd.arg(self.cargo_arguments.manifest_path());

        // Append default-feature selector, if set.
        if self.cargo_arguments.no_default_features() {
            cmd.arg("--no-default-features");
        }

        // Run cargo and verify it exited successfully.
        let output = cmd.output().map_err(|v| Error::Exec(v))?;
        if !output.status.success() {
            return Err(Error::Cargo(output.status));
        }

        // Decode output as JSON value.
        let blob = MetadataBlob::from_bytes(&output.stdout)?;

        // Parse data into a `Metadata` object.
        blob.parse(self)
    }
}

impl BuildBlob {
    fn from_str(data: &str) -> Result<Self, Error> {
        let stream = serde_json::Deserializer::from_str(data)
            .into_iter::<serde_json::Value>();
        let mut json = Vec::new();

        for v in stream {
            json.push(v.map_err(|_| Error::Json)?);
        }

        Ok(Self {
            json: json,
        })
    }

    fn from_bytes(data: &[u8]) -> Result<Self, Error> {
        Self::from_str(
            std::str::from_utf8(data).map_err(|v| Error::Unicode(v))?,
        )
    }

    // Parse all desired fields in the `Build` blob and expose them as a
    // new `Build` object.
    fn parse(&self) -> Result<Build, Error> {
        let mut success = false;
        let mut artifacts = Vec::new();

        // Iterate all reports of the build and handle the ones we are
        // interested in.
        for report in &self.json {
            let reason = match report.get("reason") {
                Some(serde_json::Value::String(v)) => v,
                _ => continue
            };

            match reason.as_str() {
                // A final `success` message tells whether the build was
                // successful. Usually, this is caught early with a negative
                // exit code, but we try to be pedantic and check this again.
                "build-finished" => {
                    if let Some(serde_json::Value::Bool(v)) = report.get("success") {
                        success = *v;
                    }
                },

                // We collect paths to all compiler-artifacts of interest. This
                // means all shared libraries and executables produced by the
                // build are collected. However, any non-production targets
                // like examples and tests are ignored.
                // We assume that a build pulls in exactly the dependencies it
                // needs. Hence, any non-statically linked artifacts will be
                // required at runtime, and thus we collect it.
                "compiler-artifact" => {
                    // See whether this artifact report is of interest. Any
                    // auxiliary targets like examples and tests are ignored.
                    let mut collect = false;
                    if let Some(serde_json::Value::Object(target)) = report.get("target") {
                        if let Some(serde_json::Value::Array(kinds)) = target.get("kind") {
                            for kind in kinds.iter() {
                                if let serde_json::Value::String(kind_str) = kind {
                                    match kind_str.as_str() {
                                        "bin" => collect = true,
                                        "cdylib" => collect = true,
                                        "dylib" => collect = true,
                                        _ => {},
                                    }
                                }
                            }
                        }
                    }

                    // If this was a report of interest, remember the artifact
                    // filenames. Unfortunately, targets like libraries will
                    // report all their artifacts combined, with no way to tell
                    // which artifact corresponds to each kind. Hence, we have
                    // filter based on filename extension. We currently ignore:
                    //
                    // - *.a, *.lib, *.rlib: Static library that will be linked
                    //   into another artifact as part of the build. Not needed
                    //   at runtime.
                    // - *.d, *.rmeta: Metadata about the build process, only
                    //   needed at compile time.
                    //
                    // XXX: Cargo should do better and tell us which file
                    //      corresponds to which target kind. We cannot predict
                    //      the filename extensions of all supported platforms
                    //      of Cargo. We do our best and blacklist artifacts we
                    //      know we are not interested in.
                    if collect {
                        if let Some(serde_json::Value::Array(filenames)) = report.get("filenames") {
                            for filename in filenames.iter() {
                                if let serde_json::Value::String(filename_str) = filename {
                                    let save = match filename_str.rsplit_once('.') {
                                        Some((_, "a")) => false,
                                        Some((_, "d")) => false,
                                        Some((_, "lib")) => false,
                                        Some((_, "rlib")) => false,
                                        Some((_, "rmeta")) => false,
                                        _ => true,
                                    };

                                    if save {
                                        artifacts.push(filename_str.clone());
                                    }
                                }
                            }
                        }
                    }
                },

                _ => {},
            }
        }

        // If Cargo never reported a successfull build, we discard all data
        // and report a generic error. Diagnostics have been rendered, so no
        // need to include more information.
        // Note that this is usually caught early by a non-0 exit code, but
        // we try to be pendantic here.
        if !success {
            return Err(Error::Cargo(Default::default()));
        }

        // Return the fully parsed build result.
        Ok(
            Build {
                artifacts: artifacts,
            }
        )
    }
}

impl<'ctx> BuildQuery<'ctx> {
    /// Request a full build operation from Cargo. This will invoke
    /// `cargo build` and parse all the cargo output into a `Build` object.
    pub fn run(&self) -> Result<Build, Error> {
        // Build the cargo-build invocation.
        let mut cmd = std::process::Command::new(cargo_command());
        cmd.args([
            "rustc",
            "--message-format=json-render-diagnostics",
        ]);

        // Append all desired environment variables.
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }

        // Append the selected features.
        for v in &self.cargo_arguments.features {
            cmd.arg("--features");
            cmd.arg(v);
        }

        // Freeze dependencies, if requested.
        if self.cargo_arguments.frozen() {
            cmd.arg("--frozen");
        }

        // Append path to the manifest.
        cmd.arg("--manifest-path");
        cmd.arg(self.cargo_arguments.manifest_path());

        // Append default-feature selector, if set.
        if self.cargo_arguments.no_default_features() {
            cmd.arg("--no-default-features");
        }

        // Select requested package.
        if let Some(ref package) = self.cargo_arguments.package {
            cmd.arg("--package");
            cmd.arg(package);
        }

        // Select requested profile.
        if let Some(ref profile) = self.cargo_arguments.profile {
            cmd.arg("--profile");
            cmd.arg(profile);
        }

        // Build for requested target.
        if let Some(ref target) = self.target {
            cmd.arg("--target");
            cmd.arg(target);
        }

        // Select requested target directory.
        if let Some(ref target_dir) = self.cargo_arguments.target_dir {
            cmd.arg("--target-dir");
            cmd.arg(target_dir);
        }

        // Always forward diagnostics to the parent error stream, so
        // the user can inspect them.
        cmd.stderr(std::process::Stdio::inherit());

        // Run cargo and verify it exited successfully.
        let output = cmd.output().map_err(|v| Error::Exec(v))?;
        if !output.status.success() {
            return Err(Error::Cargo(output.status));
        }

        // Decode output as JSON stream.
        let blob = BuildBlob::from_bytes(&output.stdout)?;

        // Parse data into a `Build` object.
        blob.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test the package name resolver and verify that it can detect
    // ambiguous names and resolve IDs directly.
    #[test]
    fn package_resolver() {
        assert!(
            matches!(
                MetadataBlob::from_str(r#"{}"#).unwrap().resolve_local_package("foo"),
                Err(Error::UnknownPackage(_)),
            )
        );
        assert_eq!(
            MetadataBlob::from_str(
                r#"{
                    "packages": [
                        { "name": "foo", "id": "foo (...)" }
                    ]
                }"#,
            ).unwrap().resolve_local_package("foo").unwrap(),
            "foo (...)",
        );
        assert!(
            matches!(
                MetadataBlob::from_str(
                    r#"{
                        "packages": [
                            { "name": "foo", "id": "foo (...)" },
                            { "name": "foo", "id": "bar (...)" }
                        ]
                    }"#,
                ).unwrap().resolve_local_package("foo"),
                Err(Error::AmbiguousPackage(_)),
            )
        );
        assert_eq!(
            MetadataBlob::from_str(
                r#"{
                    "packages": [
                        { "name": "bar (...)", "id": "foo (...)" },
                        { "name": "bar", "id": "bar (...)" }
                    ]
                }"#,
            ).unwrap().resolve_local_package("bar (...)").unwrap(),
            "bar (...)",
        );
    }

    // Test the ID collector and verify that it properly navigates the
    // dependency map, avoids circles, and only collects dependencies
    // of the root package, ignoring auxiliary dependencies.
    #[test]
    fn id_collector() {
        assert_eq!(
            MetadataBlob::from_str(
                r#"{
                    "resolve": {
                        "root": "root (...)",
                        "nodes": [
                            {
                                "id": "root (...)",
                                "deps": [
                                    {
                                        "pkg": "dep0 (...)",
                                        "dep_kinds": [
                                            { "kind": "build" },
                                            { "kind": "dev" },
                                            { "kind": null }
                                        ]
                                    },
                                    {
                                        "pkg": "nodep0 (...)",
                                        "dep_kinds": [
                                            { "kind": "build" },
                                            { "kind": "dev" }
                                        ]
                                    }
                                ]
                            },
                            {
                                "id": "dep0 (...)",
                                "deps": [
                                    {
                                        "pkg": "root (...)",
                                        "dep_kinds": [
                                            { "kind": null }
                                        ]
                                    },
                                    {
                                        "pkg": "dep1 (...)",
                                        "dep_kinds": [
                                            { "kind": null }
                                        ]
                                    }
                                ]
                            },
                            {
                                "id": "dep1 (...)",
                                "deps": [
                                    {
                                        "pkg": "root (...)",
                                        "dep_kinds": [
                                            { "kind": null }
                                        ]
                                    },
                                    {
                                        "pkg": "dep0 (...)",
                                        "dep_kinds": [
                                            { "kind": null }
                                        ]
                                    }
                                ]
                            },
                            {
                                "id": "nodep0 (...)",
                                "deps": [
                                    {
                                        "pkg": "nodep1 (...)",
                                        "dep_kinds": [
                                            { "kind": null }
                                        ]
                                    }
                                ]
                            },
                            {
                                "id": "nodep1 (...)",
                                "deps": []
                            }
                        ]
                    }
                }"#,
            ).unwrap().involved_ids("root (...)").into_iter().collect::<Vec<String>>(),
            vec![
                "dep0 (...)",
                "dep1 (...)",
                "root (...)",
            ],
        );
    }

    // Create `Metadata` from a set of predefined JSON blobs and verify it is
    // parsed as expected.
    #[test]
    fn metadata_from_json() {
        let query = MetadataQuery {
            cargo_arguments: &Arguments {
                package: Some("foobar".into()),
                ..Default::default()
            },
            target: None,
        };

        // Empty strings are invalid JSON.
        assert!(
            matches!(
                MetadataBlob::from_str("").unwrap_err(),
                Error::Json,
            ),
        );

        // Empty sets lack mandatory metadata fields and must be rejected.
        assert!(
            matches!(
                MetadataBlob::from_str("{}").unwrap().parse(&query).unwrap_err(),
                Error::Data,
            ),
        );

        // As long as our mandatory fields are present, it must parse.
        assert_eq!(
            MetadataBlob::from_str(
                r#"{
                    "packages": [
                        {
                            "id": "foobar (...)",
                            "name": "foobar",
                            "source": null
                        }
                    ],
                    "target_directory": "."
                }"#,
            ).unwrap().parse(&query).unwrap(),
            Metadata {
                android_sets: Vec::new(),
                osiris: None,
                package_id: "foobar (...)".into(),
                target_directory: ".".into(),
            },
        );
    }

    // Verify that java and kotlin configurations are properly extracted from
    // the dependency tree.
    #[test]
    fn metadata_java_kotlin() {
        let query = MetadataQuery {
            cargo_arguments: &Default::default(),
            target: None,
        };

        assert_eq!(
            MetadataBlob::from_str(
                r#"{
                    "target_directory": ".",
                    "resolve": {
                        "root": "root (...)",
                        "nodes": [
                            {
                                "id": "root (...)",
                                "deps": [
                                    { "pkg": "dep0 (...)", "dep_kinds": [{ "kind": null }] },
                                    { "pkg": "dep1 (...)", "dep_kinds": [{ "kind": null }] }
                                ]
                            },
                            {
                                "id": "dep0 (...)",
                                "deps": []
                            },
                            {
                                "id": "dep1 (...)",
                                "deps": []
                            }
                        ]
                    },
                    "packages": [
                        {
                            "id": "root (...)",
                            "manifest_path": "./Cargo.toml",
                            "metadata": null
                        },
                        {
                            "id": "dep0 (...)",
                            "manifest_path": "/foo/Cargo.toml",
                            "metadata": {
                                "android": {
                                    "java": {
                                        "source-dirs": [
                                            "foo",
                                            "dep0"
                                        ]
                                    }
                                }
                            }
                        },
                        {
                            "id": "dep1 (...)",
                            "manifest_path": "/foo/Cargo.toml",
                            "metadata": {
                                "android": {
                                    "java": {
                                        "source-dirs": [
                                            "foo",
                                            "dep1"
                                        ]
                                    },
                                    "resource-dirs": [
                                        "foo",
                                        "bar"
                                    ]
                                }
                            }
                        }
                    ]
                }"#,
            ).unwrap().parse(&query).unwrap(),
            Metadata {
                android_sets: vec![
                    MetadataAndroid {
                        java_dirs: vec![
                            "/foo/foo".into(),
                            "/foo/dep0".into(),
                        ],
                        kotlin_dirs: Vec::new(),
                        manifest_file: None,
                        resource_dirs: Vec::new(),
                    },
                    MetadataAndroid {
                        java_dirs: vec![
                            "/foo/foo".into(),
                            "/foo/dep1".into(),
                        ],
                        kotlin_dirs: Vec::new(),
                        manifest_file: None,
                        resource_dirs: vec![
                            "/foo/foo".into(),
                            "/foo/bar".into(),
                        ],
                    },
                ],
                osiris: None,
                package_id: "root (...)".into(),
                target_directory: ".".into(),
            },
        );
    }
}
