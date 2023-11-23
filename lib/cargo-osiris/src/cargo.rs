//! # Cargo Interaction
//!
//! The cargo CLI allows embedding other utilities as its subcommands. That is,
//! `cargo sub [...]` calls into `cargo-sub`. Cargo only does very basic setup
//! before invoking such external commands. For them to get any information
//! about the cargo setup, they need to call into `cargo metadata`. This module
//! provides a wrapper around that call, extracting the required information
//! from the cargo metdata JSON blob.

use crate::util;
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
    /// Collection of all java-source-directories from the crate metadata of
    /// all packages part of the main build.
    pub java_sources: Vec<std::path::PathBuf>,
    /// Collection of all kotlin-source-directories from the crate metadata of
    /// all packages part of the main build.
    pub kotlin_sources: Vec<std::path::PathBuf>,
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
        Ok(MetadataBlob {
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
    // that are involved in a normal compilation of the package specified as
    // `start` (or the root package of the workspace, if `None`).
    //
    // The `resolve.nodes` array usually contains exactly this set. However,
    // it also contains build and dev dependencies, as well as dependencies
    // of workspace packages other than the requested package. Hence, this
    // function collects exactly the required package IDs.
    fn involved_ids(&self, start: Option<&str>) -> BTreeSet<String> {
        let mut ids = BTreeSet::<String>::new();
        let mut todo = BTreeSet::<&str>::new();
        let mut depmap = BTreeMap::<&str, BTreeSet<&str>>::new();

        // Fetch the objects in the resolved dependency map of the
        // Cargo metadata blob.
        let resolve = match self.json.get("resolve") {
            Some(serde_json::Value::Object(v)) => v,
            _ => return ids,
        };
        let nodes = match resolve.get("nodes") {
            Some(serde_json::Value::Array(v)) => v,
            _ => return ids,
        };

        // Resolve the root node, unless a start node is explicitly
        // specified by the caller.
        let root = match start {
            Some(v) => v,
            None => match resolve.get("root") {
                Some(serde_json::Value::String(v)) => v.as_str(),
                _ => return ids,
            },
        };

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

        // Now start at the root package and collect all its dependencies in
        // the final set. Repeat this for each dependency, avoiding cycles.
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
        let mut java_sources = BTreeSet::new();
        let mut kotlin_sources = BTreeSet::new();
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
                let package_path = util::absdir(&manifest_path);

                // Extract all metadata we desire.
                if let Some(serde_json::Value::Object(metadata)) = pkg.get("metadata") {
                    // We use `osi` as our reserved metadata namespace for all
                    // features that have no standardized location. Once a more
                    // standard metadata namespace is found, we will use it.
                    if let Some(serde_json::Value::Object(osi)) = metadata.get("osi") {
                        // The `java` and `kotlin` configurations allow
                        // specifying an array of source-directories relative
                        // to the manifest. This allows shipping Java and
                        // Kotlin sources with a Rust crate, which are expected
                        // by the Rust package to be available in the JVM when
                        // it is loaded via JNI. It is up to the build system
                        // to decide how to make them available.
                        if let Some(serde_json::Value::Object(java)) = osi.get("java") {
                            if let Some(serde_json::Value::Array(dirs)) = java.get("source-dirs") {
                                for dir in dirs.iter() {
                                    if let serde_json::Value::String(dir_str) = dir {
                                        java_sources.insert(package_path.as_path().join(dir_str));
                                    }
                                }
                            }
                        }
                        if let Some(serde_json::Value::Object(kotlin)) = osi.get("kotlin") {
                            if let Some(serde_json::Value::Array(dirs)) = kotlin.get("source-dirs") {
                                for dir in dirs.iter() {
                                    if let serde_json::Value::String(dir_str) = dir {
                                        kotlin_sources.insert(package_path.as_path().join(dir_str));
                                    }
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
                java_sources: java_sources.into_iter().collect(),
                kotlin_sources: kotlin_sources.into_iter().collect(),
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
            "--no-deps",
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
                java_sources: Vec::new(),
                kotlin_sources: Vec::new(),
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
                                "osi": {
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
                                "osi": {
                                    "java": {
                                        "source-dirs": [
                                            "foo",
                                            "dep1"
                                        ]
                                    },
                                    "kotlin": {
                                        "source-dirs": [
                                            "foo",
                                            "bar"
                                        ]
                                    }
                                }
                            }
                        }
                    ]
                }"#,
            ).unwrap().parse(&query).unwrap(),
            Metadata {
                target_directory: ".".into(),
                java_sources: vec![
                    "/foo/dep0".into(),
                    "/foo/dep1".into(),
                    "/foo/foo".into(),
                ],
                kotlin_sources: vec![
                    "/foo/bar".into(),
                    "/foo/foo".into(),
                ],
            },
        );
    }
}
