//! # Osiris Cargo Metadata
//!
//! Osiris sources its configuration and parameters from the `metadata`
//! section of the Cargo manifest. This file implements the metadata parsers
//! defined or used by Osiris.

/// Definitions of format errors for Metadata parsing
#[derive(Debug)]
pub enum FormatError {
    /// Invalid type for the specified field
    TypeInvalid { key: String, needs: String },
    /// Supported range of the selected type was exceeded
    TypeExceeded { key: String },
    /// Mandatory key is missing
    KeyMissing { key: String },
    /// Key cannot be specified with conflicting alternatives
    KeyExclusive { key: String },
}

/// Error definitions for Osiris Metadata parsing
#[derive(Debug)]
pub enum OsirisError {
    /// Format errors
    Format(FormatError),
    /// Specified version is higher/lower than supported by this
    /// implementation
    VersionUnsupported { version: u32 },
}

/// Metadata about an application icon
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OsirisApplicationIcon {
    /// Relative path to the icon file
    pub path: Option<String>,
    /// Integer-scaling the icon applies to
    pub scale: Option<u32>,
    /// Width of the square icon in pixels before scaling
    pub size: Option<u32>,
}

/// Metadata about the application independent of the target platform
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OsirisApplication {
    /// Identifier of the application. Used to register and identify the
    /// application. Must not change over the life of the application. Only
    /// alphanumeric and `-`, `_` allowed. Non-ASCII allowed but might break
    /// external tools.
    pub id: Option<String>,
    /// Human-readable name of the application
    pub name: Option<String>,

    /// Information on the application icon, allowing for multiple alternatives
    /// that can each provide different attributes (e.g., dimensions).
    pub icons: Vec<OsirisApplicationIcon>,
}

/// Metadata about the application and library for the Android platform.
/// These are one-to-one mappings of their respective counterparts in the
/// Android SDK.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OsirisPlatformAndroid {
    pub application_id: Option<String>,
    pub namespace: Option<String>,

    pub compile_sdk: Option<u32>,
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,

    pub abis: Option<Vec<String>>,

    pub version_code: Option<u32>,
    pub version_name: Option<String>,
}

/// Metadata about the application and framework for the macOS platform
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OsirisPlatformMacos {
    pub bundle_id: Option<String>,

    pub abis: Option<Vec<String>>,
    pub min_os: Option<String>,

    pub version_code: Option<u32>,
    pub version_name: Option<String>,

    pub category: Option<String>,
}

/// Metadata specific to a platform, indexed by the name of the platform
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OsirisPlatformConfiguration {
    /// Android platform table
    Android(OsirisPlatformAndroid),
    /// Macos platform table
    Macos(OsirisPlatformMacos),
}

/// Metadata about a platform integration supported by the application
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OsirisPlatform {
    /// Custom ID of the platform integration
    pub id: String,
    /// Path to the platform integration root relative from the configuration
    pub path: Option<String>,

    /// Platform specific configuration
    pub configuration: Option<OsirisPlatformConfiguration>,
}

/// Version `1` of the Osiris metadata format.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OsirisV1 {
    /// Application table specifying properties of the application
    /// itself
    pub application: Option<OsirisApplication>,
    /// Platform table specifying all properties of the platform
    /// integration for all supported platforms
    pub platforms: Vec<OsirisPlatform>,
}

/// Osiris metadata that was embedded as `package.metadata.osiris` in a Cargo
/// manifest.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Osiris {
    /// Version `1` of the metadata format
    V1(OsirisV1),
}

/// Parse a sub-key of a JSON object as generic value
pub fn entry_from_json<'json>(
    json: &'json serde_json::Value,
    key: &str,
    path: &str,
) -> Result<Option<&'json serde_json::Value>, FormatError> {
    let serde_json::Value::Object(ref map) = *json else {
        return Err(FormatError::TypeInvalid {
            key: path.into(),
            needs: "object".into(),
        });
    };

    Ok(map.get(key))
}

/// Parse a sub-key of a JSON object as array value
pub fn array_from_json<'json>(
    json: &'json serde_json::Value,
    key: &str,
    path: &str,
) -> Result<Option<&'json Vec<serde_json::Value>>, FormatError> {
    let Some(entry) = entry_from_json(json, key, path)? else {
        return Ok(None);
    };

    entry.as_array().ok_or_else(
        || FormatError::TypeInvalid {
            key: format!("{}.{}", path, key),
            needs: "array".into(),
        },
    ).map(|v| Some(v))
}

/// Parse a sub-key of a JSON object as array of strings
pub fn array_str_from_json<'json>(
    json: &'json serde_json::Value,
    key: &str,
    path: &str,
) -> Result<Option<Vec<&'json str>>, FormatError> {
    let Some(array) = array_from_json(json, key, path)? else {
        return Ok(None);
    };

    let mut acc = Vec::new();

    for v in array {
        let Some(v_str) = v.as_str() else {
            return Err(FormatError::TypeInvalid {
                key: format!("{}.{}.[]", path, key),
                needs: "string".into(),
            });
        };

        acc.push(v_str);
    }

    Ok(Some(acc))
}

/// Parse a sub-key of a JSON object as a string value
pub fn str_from_json<'json>(
    json: &'json serde_json::Value,
    key: &str,
    path: &str,
) -> Result<Option<&'json str>, FormatError> {
    let Some(entry) = entry_from_json(json, key, path)? else {
        return Ok(None);
    };

    entry.as_str().ok_or_else(
        || FormatError::TypeInvalid {
            key: format!("{}.{}", path, key),
            needs: "string".into(),
        },
    ).map(|v| Some(v))
}

/// Parse a sub-key of a JSON object as a number value
pub fn num_from_json<'json>(
    json: &'json serde_json::Value,
    key: &str,
    path: &str,
) -> Result<Option<&'json serde_json::value::Number>, FormatError> {
    let Some(entry) = entry_from_json(json, key, path)? else {
        return Ok(None);
    };

    entry.as_number().ok_or_else(
        || FormatError::TypeInvalid {
            key: format!("{}.{}", path, key),
            needs: "number".into(),
        },
    ).map(|v| Some(v))
}

/// Parse a sub-key of a JSON object as a u32 value
pub fn u32_from_json<'json>(
    json: &'json serde_json::Value,
    key: &str,
    path: &str,
) -> Result<Option<u32>, FormatError> {
    let Some(entry) = num_from_json(json, key, path)? else {
        return Ok(None);
    };

    let Some(entry_u64) = entry.as_u64() else {
        return Err(FormatError::TypeExceeded {
            key: format!("{}.{}", path, key),
        });
    };

    u32::try_from(entry_u64).map_err(|_| {
        FormatError::TypeExceeded { key: format!("{}.{}", path, key) }
    }).map(|v| Some(v))
}

fn osiris_android_from_json(
    json: &serde_json::Value,
) -> Result<OsirisPlatformAndroid, OsirisError> {
    let v_application_id = str_from_json(json, "application-id", "osiris.platforms.[].android")?;
    let v_namespace = str_from_json(json, "namespace", "osiris.platforms.[].android")?;
    let v_compile_sdk = u32_from_json(json, "compile-sdk", "osiris.platforms.[].android")?;
    let v_min_sdk = u32_from_json(json, "min-sdk", "osiris.platforms.[].android")?;
    let v_target_sdk = u32_from_json(json, "target-sdk", "osiris.platforms.[].android")?;
    let v_abis = array_str_from_json(json, "abis", "osiris.platforms.[].android")?;
    let v_version_code = u32_from_json(json, "version-code", "osiris.platforms.[].android")?;
    let v_version_name = str_from_json(json, "version-name", "osiris.platforms.[].android")?;

    Ok(OsirisPlatformAndroid {
        application_id: v_application_id.map(|v| v.into()),
        namespace: v_namespace.map(|v| v.into()),

        compile_sdk: v_compile_sdk,
        min_sdk: v_min_sdk,
        target_sdk: v_target_sdk,

        abis: v_abis.map(|v| v.iter().map(|v| v.to_string()).collect()),

        version_code: v_version_code,
        version_name: v_version_name.map(|v| v.into()),
    })
}

fn osiris_macos_from_json(
    json: &serde_json::Value,
) -> Result<OsirisPlatformMacos, OsirisError> {
    let v_bundle_id = str_from_json(json, "bundle-id", "osiris.platforms.[].macos")?;
    let v_abis = array_str_from_json(json, "abis", "osiris.platforms.[].macos")?;
    let v_min_os = str_from_json(json, "min-os", "osiris.platforms.[].macos")?;
    let v_version_code = u32_from_json(json, "version-code", "osiris.platforms.[].macos")?;
    let v_version_name = str_from_json(json, "version-name", "osiris.platforms.[].macos")?;
    let v_category = str_from_json(json, "category", "osiris.platforms.[].macos")?;

    Ok(OsirisPlatformMacos {
        bundle_id: v_bundle_id.map(|v| v.into()),

        abis: v_abis.map(|v| v.iter().map(|v| v.to_string()).collect()),
        min_os: v_min_os.map(|v| v.into()),

        version_code: v_version_code,
        version_name: v_version_name.map(|v| v.into()),

        category: v_category.map(|v| v.into()),
    })
}

/// Parse Osiris metadata from its JSON representation
pub fn osiris_from_json(
    json: &serde_json::Value,
) -> Result<Osiris, OsirisError> {
    // Figure out the metadata version.
    let _version = match u32_from_json(json, "version", "osiris")? {
        None => Ok(None),
        Some(1) => Ok(Some(1)),
        Some(v) => Err(OsirisError::VersionUnsupported { version: v }),
    }?;

    // Create the top-level object and parse everything into it. Only
    // version 1 is defined so far.
    let mut osi = OsirisV1 {
        application: None,
        platforms: Vec::new(),
    };

    // Extract the application data
    if let Some(json_application) = entry_from_json(json, "application", "osiris")? {
        let v_id = str_from_json(json_application, "id", "osiris.application")?;
        let v_name = str_from_json(json_application, "name", "osiris.application")?;

        let mut osi_application = OsirisApplication {
            id: v_id.map(|v| v.into()),
            name: v_name.map(|v| v.into()),

            icons: Vec::new(),
        };

        if let Some(json_icons) = array_from_json(json_application, "icons", "osiris.application")? {
            for json_icon in json_icons {
                let v_path = str_from_json(json_icon, "path", "osiris.application.icons.[]")?;
                let v_scale = u32_from_json(json_icon, "scale", "osiris.application.icons.[]")?;
                let v_size = u32_from_json(json_icon, "size", "osiris.application.icons.[]")?;

                osi_application.icons.push(OsirisApplicationIcon {
                    path: v_path.map(|v| v.into()),
                    scale: v_scale,
                    size: v_size,
                });
            }
        }

        osi.application = Some(osi_application);
    }

    // Extract the platforms data
    if let Some(json_platforms) = array_from_json(json, "platforms", "osiris")? {
        for json_platform in json_platforms {
            let v_id = str_from_json(json_platform, "id", "osiris.platforms.[]")?
                .ok_or_else(|| FormatError::KeyMissing { key: "osiris.platforms.[].id".into() })?;
            let v_path = str_from_json(json_platform, "path", "osiris.platforms.[]")?;

            let v_android = entry_from_json(json_platform, "android", "osiris.platforms.[]")?;
            let v_macos = entry_from_json(json_platform, "macos", "osiris.platforms.[]")?;
            let v_configuration = match (v_android, v_macos) {
                (None, None) => Ok(None),
                (Some(v), None) => {
                    osiris_android_from_json(v).map(
                        |v| Some(OsirisPlatformConfiguration::Android(v)),
                    )
                },
                (None, Some(v)) => {
                    osiris_macos_from_json(v).map(
                        |v| Some(OsirisPlatformConfiguration::Macos(v)),
                    )
                },
                _ => Err(FormatError::KeyExclusive { key: "osiris.platforms.[].{android,macos}".into() }.into()),
            }?;

            let osi_platform = OsirisPlatform {
                id: v_id.into(),
                path: v_path.map(|v| v.into()),
                configuration: v_configuration,
            };

            osi.platforms.push(osi_platform);
        }
    }

    Ok(Osiris::V1(osi))
}

impl core::fmt::Display for FormatError {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        match self {
            FormatError::TypeInvalid { key, needs } => fmt.write_fmt(core::format_args!("Entry requires a value of a different type: `{}` needs type `{}`", key, needs)),
            FormatError::TypeExceeded { key } => fmt.write_fmt(core::format_args!("Entry exceeds the maximum supported range for its type: {}", key)),
            FormatError::KeyMissing { key } => fmt.write_fmt(core::format_args!("Required entry was not specified: {}", key)),
            FormatError::KeyExclusive { key } => fmt.write_fmt(core::format_args!("Exclusive entry was specified with conflicts: {}", key)),
        }
    }
}

impl core::fmt::Display for OsirisError {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        match self {
            OsirisError::Format(e) => fmt.write_fmt(core::format_args!("Osiris metadata format error: {}", e)),
            OsirisError::VersionUnsupported { version } => fmt.write_fmt(core::format_args!("Version is not supported: {}", version)),
        }
    }
}

impl core::convert::From<FormatError> for OsirisError {
    fn from(v: FormatError) -> Self {
        OsirisError::Format(v)
    }
}
