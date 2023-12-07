//! # Cargo Interaction
//!
//! Provide Cargo sub-command executors and parsers. These allow invoking Cargo
//! subcommands programmatically, parsing the output into machine-readable
//! types.

use crate::misc;
use std::collections::{BTreeMap, BTreeSet};

/// ## Metadata Extraction Errors
///
/// This error-enum describes the possible errors from the metadata extraction
/// helper. See each error-code for details on when it is raised.
#[derive(Debug)]
pub enum Error {
    /// Execution of `cargo` could not commence.
    Exec(std::io::Error),
    /// `cargo` exited without success.
    Cargo(std::process::ExitStatus),
    /// Unicode decoding error.
    Unicode(std::str::Utf8Error),
    /// JSON decoding error.
    Json,
    /// Unknown package reference
    UnknownPackage(String),
    /// Ambiguous package reference
    AmbiguousPackage(String),
    /// Data decoding error.
    Data,
}

/// ## Reduced Cargo Metadata
///
/// This struct represents the reduced cargo metadata with only the bits that
/// are required by us.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Metadata {
    /// Target directory of the package build.
    pub target_directory: String,
    /// Collection of all android-resource-directories from the crate metadata
    /// of all packages part of the main build.
    pub android_resource_dirs: Vec<std::path::PathBuf>,
    /// Collection of all android-java-directories from the crate metadata of
    /// all packages part of the main build.
    pub android_java_dirs: Vec<std::path::PathBuf>,
}

// Intermediate state after cargo-metadata returned, but the blob was not yet
// parsed into the metadata object.
#[derive(Debug)]
struct MetadataBlob {
    pub json: serde_json::Value,
}

/// ## Metadata query parameters
///
/// This open-coded structure provides the parameters for a query to
/// `cargo-metadata`. It is to be filled in by the caller.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MetadataQuery {
    /// Path to the workspace directory (or package directory) where
    /// `Cargo.toml` resides (preferably an absolute path).
    pub workspace: std::path::PathBuf,
    /// Name of the main package in the workspace (if `None`, the workspace
    /// root is used).
    pub main_package: Option<String>,
    /// The target platform to compile for (if `None`, a generic request for
    /// all possible targets is performed).
    pub target: Option<String>,
}

/// ## Reduced Cargo Build Output
///
/// This struct represents the reduced cargo build output with only the bits
/// that are required by us.
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

/// ## Build query parameters
///
/// This open-coded structure provides the parameters for a query to
/// `cargo-build`. It is to be filled in by the caller.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BuildQuery {
    /// Whether to enable default features.
    pub default_features: bool,
    /// Array of features to enable.
    pub features: Vec<String>,
    /// The build profile to use.
    pub profile: Option<String>,
    /// The target platform to compile for.
    pub target: Option<String>,
    /// Path to the workspace directory (or package directory) where
    /// `Cargo.toml` resides (preferably an absolute path).
    pub workspace: std::path::PathBuf,
}

// ## Return Cargo binary to use
//
// Return the Cargo command to use for invocations of Cargo. This will
// look at the `CARGO` environment variable first, and if unset use the
// default `cargo` command.
//
// Note that Cargo sub-commands get the `CARGO` environment variable set
// unconditionally, and thus ensure that the correct toolchain is used.
fn cargo_command() -> std::ffi::OsString {
    std::env::var_os("CARGO").unwrap_or("cargo".into())
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
            let name = match pkg.get("name") {
                Some(serde_json::Value::String(v)) => v,
                _ => continue
            };
            let id = match pkg.get("id") {
                Some(serde_json::Value::String(v)) => v,
                _ => continue
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
                if key == name {
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
    fn involved_ids(&self, start: Option<&str>) -> BTreeSet<String> {
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

        // Find the root nodes. If the caller specifies them, we use them.
        // Otherwise, we use the resolved root package. If that is not
        // suitable, we use the default workspace.
        //
        // If the resolved root nodes are an empty set, there is nothing
        // to do and we yield an empty set to the caller.
        //
        // XXX: This should be extended to allow the caller to specify more
        //      than one root package, and to select whether to compile the
        //      entire workspace.
        if let Some(root_str) = start {
            roots.insert(root_str);
        } else if let Some(serde_json::Value::String(root_str)) = resolve.get("root") {
            roots.insert(root_str);
        } else if let Some(serde_json::Value::Array(dirs)) = self.json.get("workspace_default_members") {
            for dir in dirs.iter() {
                if let serde_json::Value::String(dir_str) = dir {
                    roots.insert(dir_str);
                }
            }
        }

        if roots.is_empty() {
            return ids;
        }

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
        let data_target_directory = self.json.get("target_directory").ok_or(Error::Data)?
            .as_str().ok_or(Error::Data)?
            .to_string();

        // Walk the dependency graph and collect all packages that are part of
        // this compilation. We have to do this, since only the dependency
        // graph is affected by target-filtering and feature-selection, and we
        // want to avoid any build or dev dependencies.
        let ids = self.involved_ids(query.main_package.as_deref());

        // Now walk the package list and extract all data we desire.
        let mut java_dirs = BTreeSet::new();
        let mut res_dirs = BTreeSet::new();
        if let Some(serde_json::Value::Array(packages)) = self.json.get("packages") {
            for pkg in packages.iter() {
                // Skip any packages that we are not interested in.
                let id = match pkg.get("id") {
                    Some(serde_json::Value::String(v)) => v,
                    _ => continue
                };
                if !ids.contains(id) {
                    continue;
                }

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
                                        java_dirs.insert(package_path.as_path().join(dir_str));
                                    }
                                }
                            }
                        }
                        if let Some(serde_json::Value::Array(dirs)) = android.get("resource-dirs") {
                            for dir in dirs.iter() {
                                if let serde_json::Value::String(dir_str) = dir {
                                    res_dirs.insert(package_path.as_path().join(dir_str));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Return the parsed `Metadata` object.
        Ok(
            Metadata {
                target_directory: data_target_directory,
                android_java_dirs: java_dirs.into_iter().collect(),
                android_resource_dirs: res_dirs.into_iter().collect(),
            }
        )
    }
}

impl MetadataQuery {
    /// ## Query metadata from Cargo
    ///
    /// Invoke `cargo metadata` and parse all the cargo metadata into the
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

        // Append path to the manifest.
        let mut path_manifest = std::path::PathBuf::new();
        path_manifest.push(&self.workspace);
        path_manifest.push("Cargo.toml");
        cmd.arg("--manifest-path");
        cmd.arg(&path_manifest);

        // Run cargo and verify it exited successfully.
        let output = cmd.output().map_err(|v| Error::Exec(v))?;
        if !output.status.success() {
            return Err(Error::Cargo(output.status));
        }

        // Decode output as JSON value.
        let blob = MetadataBlob::from_bytes(&output.stdout)?;

        // Parse data into a `Metadata` object.
        blob.parse(&self)
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

impl BuildQuery {
    /// ## Query build results from Cargo
    ///
    /// Invoke `cargo build` and parse all the cargo output into a
    /// `Build` object.
    pub fn run(&self) -> Result<Build, Error> {
        // Build the cargo-build invocation.
        let mut cmd = std::process::Command::new(cargo_command());
        cmd.args([
            "build",
            "--message-format=json-render-diagnostics",
            "--quiet",
        ]);

        // Append all selected features.
        for v in &self.features {
            cmd.arg("--features");
            cmd.arg(v);
        }

        // Append path to the manifest.
        let mut path_manifest = std::path::PathBuf::new();
        path_manifest.push(&self.workspace);
        path_manifest.push("Cargo.toml");
        cmd.arg("--manifest-path");
        cmd.arg(&path_manifest);

        // Disable default features, if requested.
        if !self.default_features {
            cmd.arg("--no-default-features");
        }

        // Select requested profile.
        if let Some(ref profile) = self.profile {
            cmd.arg("--profile");
            cmd.arg(profile);
        }

        // Build for requested target.
        if let Some(ref target) = self.target {
            cmd.arg("--target");
            cmd.arg(target);
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
            ).unwrap().involved_ids(None).into_iter().collect::<Vec<String>>(),
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
            workspace: ".".into(),
            main_package: None,
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
                    "target_directory": "."
                }"#,
            ).unwrap().parse(&query).unwrap(),
            Metadata {
                target_directory: ".".into(),
                android_java_dirs: Vec::new(),
                android_resource_dirs: Vec::new(),
            },
        );
    }

    // Verify that java and kotlin configurations are properly extracted from
    // the dependency tree.
    #[test]
    fn metadata_java_kotlin() {
        let query = MetadataQuery {
            workspace: ".".into(),
            main_package: None,
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
                target_directory: ".".into(),
                android_java_dirs: vec![
                    "/foo/dep0".into(),
                    "/foo/dep1".into(),
                    "/foo/foo".into(),
                ],
                android_resource_dirs: vec![
                    "/foo/bar".into(),
                    "/foo/foo".into(),
                ],
            },
        );
    }
}
