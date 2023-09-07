//! # Cargo Interaction
//!
//! The cargo CLI allows embedding other utilities as its subcommands. That is,
//! `cargo sub [...]` calls into `cargo-sub`. Cargo only does very basic setup
//! before invoking such external commands. For them to get any information
//! about the cargo setup, they need to call into `cargo metadata`. This module
//! provides a wrapper around that call, extracting the required information
//! from the cargo metdata JSON blob.

/// Metadata Extraction Errors
///
/// This error-enum describes the possible errors from the metadata extraction
/// helper. See each error-code for details on when it is raised.
#[derive(Debug)]
pub enum Error {
    /// Execution of `cargo` could not commence.
    Exec(std::io::Error),
    /// `cargo` exited without success.
    Cargo,
    /// Unicode decoding error.
    Unicode(std::str::Utf8Error),
    /// JSON decoding error.
    Json,
    /// Data decoding error.
    Data,
}

/// Reduced Cargo Metadata
///
/// This struct represents the reduced cargo metadata with only the bits that
/// are required by the crate.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Metadata {
    pub target_directory: String,
}

impl Metadata {
    /// Create from Cargo metadata object
    ///
    /// Create the `Metadata` object from the JSON-encoded Cargo metadata
    /// object, obtained by the caller by querying Cargo.
    fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        // Decode output as JSON value.
        let data = std::str::from_utf8(bytes).map_err(|v| Error::Unicode(v))?;
        let json: serde_json::Value = serde_json::from_str(data).map_err(|_| Error::Json)?;

        // Extract `target_directory` from the blob. It is a mandatory field.
        let data_target_directory = json.get("target_directory").ok_or(Error::Data)?
            .as_str().ok_or(Error::Data)?
            .to_string();

        // Return the parsed `Metadata` object.
        Ok(
            Metadata {
                target_directory: data_target_directory,
            }
        )
    }

    /// Query metadata from Cargo
    ///
    /// Invoke `cargo metadata` and parse all the cargo metadata into the
    /// `Metadata` object. Only the bits required by the crate are fetched,
    /// everything else is ignored.
    pub fn query(path: &dyn AsRef<std::path::Path>) -> Result<Self, Error> {
        // Get the path to cargo via the `CARGO` env var. This is always set by
        // cargo when running external sub-commands. If unset, it means this is
        // called outside cargo and we use the default.
        let cargo = std::env::var_os("CARGO").unwrap_or("cargo".into());

        // Build the cargo-metadata invocation.
        let mut cmd = std::process::Command::new(cargo);
        cmd.args([
            "metadata",
            "--format-version=1",
            "--no-deps",
            "--offline",
            "--quiet",
        ]);

        // Append path to the manifest.
        let mut path_manifest = std::path::PathBuf::new();
        path_manifest.push(path.as_ref());
        path_manifest.push("Cargo.toml");
        cmd.arg("--manifest-path");
        cmd.arg(&path_manifest);

        // Run cargo and verify it exited successfully.
        let output = cmd.output().map_err(|v| Error::Exec(v))?;
        if !output.status.success() {
            return Err(Error::Cargo);
        }

        // Parse data into a `Metadata` object.
        Self::from_json(&output.stdout)
    }
}
