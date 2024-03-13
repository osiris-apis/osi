//! # Cargo Interaction
//!
//! Provide Cargo sub-command executors and parsers. These allow invoking Cargo
//! subcommands programmatically, parsing the output into machine-readable
//! types.

use crate::{md, misc};
use std::collections::{BTreeMap, BTreeSet};

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
    MdOsiris(md::OsirisError),
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
    pub osiris: Option<md::Osiris>,
    /// Package ID of the target package
    pub package_id: String,
    /// Package name of the target package
    pub package_name: String,
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

/// Information on a single artifact produced by a build query.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BuildArtifact {
    /// Whether this is an executable.
    pub is_executable: bool,
    /// Package ID of the origin of this artifact.
    pub package_id: String,
    /// Path to the artifact.
    pub path: String,
}

/// Output of a `cargo build` run with only relevant pieces retained.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Build {
    /// List of artifacts produced by the build.
    pub artifacts: Vec<BuildArtifact>,
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
    /// Additional compilation environment variables to set
    pub cfgs: Vec<(String, Option<String>)>,
    /// Crate type to build
    pub crate_type: Option<String>,
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
            Error::MdOsiris(e) => fmt.write_fmt(core::format_args!("Cannot parse Osiris metadata: {}", e)),
        }
    }
}

impl core::convert::From<md::OsirisError> for Error {
    fn from(v: md::OsirisError) -> Self {
        Error::MdOsiris(v)
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

    // Parse all desired fields in the manifest blob and expose them as a
    // new Metadata object.
    fn parse(&self, query: &MetadataQuery) -> Result<Metadata, Error> {
        // Extract `target_directory` from the blob. It is a mandatory field.
        let v_target_directory = self.json.get("target_directory").ok_or(Error::Data)?
            .as_str().ok_or(Error::Data)?
            .to_string();

        // If no package name was specified, find the root package in the metadata. If none
        // is present, raise an error to the caller.
        let package_raw = match &query.cargo_arguments.package {
            None => match self.json.get("resolve") {
                Some(serde_json::Value::Object(v)) => match v.get("root") {
                    Some(serde_json::Value::String(v)) => Ok(v),
                    _ => Err(Error::NoPackage),
                },
                _ => Err(Error::NoPackage),
            },
            Some(v) => Ok(v),
        }?;
        let package_id = self.resolve_local_package(package_raw)?;

        // Walk the dependency graph and collect all packages that are part of
        // this compilation. We have to do this, since only the dependency
        // graph is affected by target-filtering and feature-selection, and we
        // want to avoid any build or dev dependencies.
        let ids = self.involved_ids(&package_id);

        // Now walk the package list and extract all data we desire.
        let mut android_sets = Vec::new();
        let mut o_package_name = None;
        let mut pkgmd_osi = None;
        if let Some(serde_json::Value::Array(packages)) = self.json.get("packages") {
            for pkg in packages.iter() {
                let mut java_dirs = Vec::new();
                let mut kotlin_dirs = Vec::new();
                let mut manifest_file = None;
                let mut res_dirs = Vec::new();

                // Get the package ID and remember whether it is the root.
                let id = match pkg.get("id") {
                    Some(serde_json::Value::String(v)) => v,
                    _ => continue
                };
                let is_root = *id == package_id;

                // Remember the original package-name of the root package.
                if is_root {
                    if let Some(serde_json::Value::String(v)) = pkg.get("name") {
                        o_package_name = Some(v.into());
                    }
                }

                // Skip packages we are not interested in.
                if !ids.contains(id) {
                    continue;
                }

                // Get the absolute path to the package root. We need this to
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
                        if let Some(v) = metadata.get("osiris") {
                            pkgmd_osi = Some(md::osiris_from_json(v)?);
                        }
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

        let package_name = o_package_name.ok_or_else(
            || Error::Data,
        )?;

        // Return the parsed `Metadata` object.
        Ok(
            Metadata {
                android_sets: android_sets,
                osiris: pkgmd_osi,
                package_id: package_id,
                package_name: package_name,
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

                    // Remember the package-id of every artifact we are
                    // interested in, so we can track it properly.
                    let o_package_id = match (collect, report.get("package_id")) {
                        (true, Some(serde_json::Value::String(id))) => Some(id),
                        _ => None,
                    };

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
                    if let Some(package_id) = o_package_id {
                        let o_executable = match report.get("executable") {
                            Some(serde_json::Value::String(v)) => Some(v),
                            _ => None,
                        };

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
                                        artifacts.push(BuildArtifact {
                                            is_executable: Some(filename_str) == o_executable,
                                            package_id: package_id.into(),
                                            path: filename_str.clone(),
                                        });
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
            "--lib",
            "--message-format=json-render-diagnostics",
        ]);

        // Append all desired environment variables.
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }

        // Select the crate-type to build.
        if let Some(ref v) = self.crate_type {
            cmd.arg("--crate-type");
            cmd.arg(v);
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

        // Separate Cargo options from rustc options
        cmd.arg("--");

        // Append compilation environments. This must either be `--cfg KEY` or
        // `--cfg KEY="VALUE"`. The key is limited to valid rust identifiers
        // and the value must be a valid rust (raw) string literal. We do not
        // verify this, and we leave it to the caller to decide how to provide
        // the data.
        //
        // XXX: Ideally, we would verify the input enough to ensure it cannot
        //      produce bogus unintended results. This is not critical, but
        //      would certainly be nice.
        for (key, o_value) in &self.cfgs {
            cmd.arg("--cfg");
            if let Some(value) = o_value {
                cmd.arg(format!("{}={}", key, value));
            } else {
                cmd.arg(key);
            }
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
                package_name: "foobar".into(),
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
                            "metadata": null,
                            "name": "root"
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
                            },
                            "name": "dep0"
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
                            },
                            "name": "dep1"
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
                package_name: "root".into(),
                target_directory: ".".into(),
            },
        );
    }
}
