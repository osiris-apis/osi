//! # TOML Configuration
//!
//! This is the Rust representation of the TOML-encoded configuration used by
//! the build system.

use serde;
use toml;

/// ## Parser Error
///
/// The error-enum reported by the configuration parser for a single parse
/// operation.
#[derive(Debug)]
pub enum Error {
    /// Reading from the file system failed with the given I/O error.
    File(std::io::Error),
    /// Invalid TOML syntax (syntactical error).
    Toml(String, Option<core::ops::Range<usize>>),
    /// Invalid TOML content (structural error).
    Data(String, Option<core::ops::Range<usize>>),
    /// Unsupported format version number.
    Version(u32),
}

impl Error {
    fn from_toml_syntax(e: &toml::de::Error) -> Self {
        Self::Toml(e.message().to_string(), e.span())
    }

    fn from_toml_data(e: &toml::de::Error) -> Self {
        Self::Data(e.message().to_string(), e.span())
    }
}

/// ## Raw Application Table
///
/// Sub-type of `Raw` representing the `Application` table. This contains all
/// configuration regarding the rust application.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RawApplication {
    /// Identifier of the application. Used to register and identify the
    /// application. Must not change over the life of the application. Only
    /// alphanumeric and `-`, `_` allowed. Non-ASCII allowed but might break
    /// external tools.
    pub id: Option<String>,
    /// Human-readable name of the application.
    pub name: Option<String>,
    /// Path to the application root relative from the configuration. Defaults
    /// to `.`.
    pub path: Option<String>,

    /// Name of the Cargo package that implements the application entry-point.
    pub package: Option<String>,
}

/// ## Android Platform Table
///
/// Sub-type of `RawPlatform` defining all the Android platform integration
/// options and related definitions.
///
/// The options in this table are one-to-one mappings of their equivalents
/// in the Android Application SDK.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RawPlatformAndroid {
    pub application_id: Option<String>,
    pub namespace: Option<String>,

    pub compile_sdk: Option<u32>,
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,

    pub abis: Option<Vec<String>>,
    pub ndk_level: Option<u32>,

    pub version_code: Option<u32>,
    pub version_name: Option<String>,

    pub sdk_path: Option<String>,
}

/// ## Platform Union
///
/// Sub-type of `RawPlatform` defining the union over all possible platform
/// specific configuration types.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RawPlatformConfiguration {
    /// Android-platform table
    Android(RawPlatformAndroid),
}

/// ## Raw Platform Table
///
/// Sub-type of `Raw` representing the `Platform` table. This contains all
/// configuration of the platform integration modules.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RawPlatform {
    /// Custom ID of the platform integration.
    pub id: String,
    /// Path to the platform integration root relative from the configuration.
    pub path: Option<String>,

    /// Platform specific configuration.
    #[serde(flatten)]
    pub configuration: Option<RawPlatformConfiguration>,
}

/// ## Raw Content
///
/// This type contains the raw content as parsed by `toml` and converted into
/// Rust types via `serde`.
///
/// Note that content of the type is not verified other than for syntactic
/// correctness required by the given types. Semantic correctness needs to
/// be verified by the caller.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Raw {
    /// Version of the format. Only version `1` is currently supported.
    pub version: u32,

    /// Application table specifying properties of the application itself.
    pub application: Option<RawApplication>,
    /// Platform table specifying all properties of the platform integration.
    #[serde(default)]
    pub platform: Vec<RawPlatform>,
}

impl Raw {
    // Parse configuration from an in-memory TOML representation.
    fn parse_toml(table: toml::Table) -> Result<Self, Error> {
        // Parse TOML data into structured types via serde.
        let raw = <Self as serde::Deserialize>::deserialize(table)
            .map_err(|v| Error::from_toml_data(&v))?;

        // We only support version '1'. Any other version number is explicitly
        // defined to be incompatible, so fail parsing.
        //
        // Note that we do support unknown-fields. Hence, it is valid to add
        // more fields to version '1' without breaking backwards compatibility.
        // However, they will be silently ignored by older implementations.
        match raw.version {
            1 => Ok(raw),
            _ => Err(Error::Version(raw.version)),
        }
    }

    // Parse configuration from an in-memory string.
    fn parse_str(content: &str) -> Result<Self, Error> {
        content.parse::<toml::Table>()
            .map_err(|v| Error::from_toml_syntax(&v))
            .and_then(|v| Self::parse_toml(v))
    }

    /// ## Parse from file-system
    ///
    /// Open the specified file and parse it. The content is verified and
    /// invalid formats are refused. The file is completely parsed into memory
    /// and then closed again before the function returns.
    pub fn parse_path(path: &dyn AsRef<std::path::Path>) -> Result<Self, Error> {
        std::fs::read_to_string(path)
            .map_err(|v| Error::File(v))
            .and_then(|v| Self::parse_str(&v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify basic parsing of `Raw`
    //
    // Parse a minimal raw manifest into `Raw` to have a base-level test for
    // the parsing capabilities. No complex content verification is done.
    #[test]
    fn raw_parse_minimal() {
        let s = "version = 1";

        let _ = Raw::parse_str(s).unwrap();
    }

    // Verify unknown versions in `Raw`
    //
    // Parse a high version number and verify that the raw content parser
    // will fail.
    #[test]
    fn raw_parse_unknown_version() {
        let s = "version = 12345678";

        assert!(
            matches!(
                Raw::parse_str(s).unwrap_err(),
                Error::Version(12345678),
            ),
        );
    }

    // Test invalid TOML syntax
    #[test]
    fn raw_parse_invalid_toml() {
        let s = "version = =";

        assert!(
            matches!(
                Raw::parse_str(s).unwrap_err(),
                Error::Toml(_, _),
            ),
        );
    }

    // Test invalid TOML data
    #[test]
    fn raw_parse_invalid_data() {
        let s = "version_ = 1";

        assert!(
            matches!(
                Raw::parse_str(s).unwrap_err(),
                Error::Data(_, _),
            ),
        );
    }

    // Test invalid filesystem path
    #[test]
    fn raw_parse_invalid_path() {
        assert!(
            matches!(
                Raw::parse_path(&"/<invalid-osiris-path>").unwrap_err(),
                Error::File(_),
            ),
        );
    }
}
