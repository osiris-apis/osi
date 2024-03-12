//! # Configuration
//!
//! This provides structured and verified configuration data for the build
//! system. Unlike the data-based modules, this module provides all
//! configuration in an easy to use format, with helper functions, and
//! with defaults filled in. This module is purely meant to consume the
//! build system configuration. It is not meant to produce any configuration
//! or reproduce the exact layout of the original input. Use the data-based
//! modules for this.
//!
//! Most of the configuration mirrors the configuration provided by the data
//! parsers, but has defaults filled in, in case no value was provided, or
//! refuse validation if a mandatory key was missing. See the data parsers
//! for documentation on most keys, and how they are to be interpreted.

use crate::{cargo, lib, misc};
use std::collections::BTreeMap;

/// Enumeration of all errors that can occur when assembling the configuration
/// data from source material.
#[derive(Debug)]
pub enum Error {
    /// Specified key is required, but missing
    MissingKey(&'static str),
    /// Duplicate platform IDs
    DuplicatePlatform(String),
}

/// Metadata on a particular icon instance.
pub struct ConfigIcon {
    pub path: String,
    pub scale: u32,
    pub size: u32,
}

/// Android specific configuration for a platform integration.
pub struct ConfigPlatformAndroid {
    pub application_id: String,
    pub namespace: String,

    pub compile_sdk: u32,
    pub min_sdk: u32,
    pub target_sdk: u32,

    pub abis: Vec<String>,

    pub version_code: u32,
    pub version_name: String,
}

/// MacOS specific configuration for a platform integration.
pub struct ConfigPlatformMacos {
    pub bundle_id: String,
    pub namespace: Option<String>,

    pub abis: Vec<String>,
    pub min_os: String,

    pub version_code: u32,
    pub version_name: String,

    pub category: String,
}

/// Union for platform specific configuration that is part of a platform
/// integration configuration.
pub enum ConfigPlatformConfiguration {
    Android(ConfigPlatformAndroid),
    Macos(ConfigPlatformMacos),
}

/// Configuration for a specific platform integration.
pub struct ConfigPlatform {
    /// Absolute path to the platform root.
    pub path_platform: std::path::PathBuf,

    /// Platform ID
    pub id: String,
    /// Symbolized Platform ID
    pub id_symbol: String,

    /// Platform-specific configuration
    pub configuration: ConfigPlatformConfiguration,
}

/// Root object of the build system configuration. This contains sanitized
/// data with defaults filled in.
pub struct Config {
    /// Absolute path to the application root.
    pub path_application: std::path::PathBuf,
    /// Absolute path to the Cargo manifest.
    pub path_manifest: std::path::PathBuf,
    /// Absolute path to the Cargo target directory.
    pub path_target: std::path::PathBuf,

    /// Application ID
    pub id: String,
    /// Symbolized Application ID
    pub id_symbol: String,
    /// Application name
    pub name: String,

    /// Icon information
    pub icons: Vec<ConfigIcon>,

    /// Platform Configurations
    pub platforms: BTreeMap<String, ConfigPlatform>,
    /// Default platform configurations
    pub platform_defaults: BTreeMap<String, ConfigPlatform>,
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        match self {
            Self::MissingKey(key) => fmt.write_fmt(core::format_args!("Missing mandatory configuration for: {}", key)),
            Self::DuplicatePlatform(id) => fmt.write_fmt(core::format_args!("Duplicate platform configuration for ID: {}", id)),
        }
    }
}

impl Config {
    fn add_default_platforms(&mut self) {
        self.platform_defaults.insert(
            "android".to_string(),
            ConfigPlatform {
                path_platform: (&self.path_application).join("platform/android"),

                id: "android".to_string(),
                id_symbol: "android".to_string(),

                configuration: ConfigPlatformConfiguration::Android(
                    ConfigPlatformAndroid {
                        application_id: "com.example.unknown".to_string(),
                        namespace: "com.example".to_string(),

                        compile_sdk: 31,
                        min_sdk: 31,
                        target_sdk: 31,

                        abis: ["armeabi-v7a", "arm64-v8a", "x86", "x86_64"]
                            .iter().map(|v| v.to_string()).collect(),

                        version_code: 1,
                        version_name: "0.1.0".to_string(),
                    },
                ),
            },
        );

        self.platform_defaults.insert(
            "macos".to_string(),
            ConfigPlatform {
                path_platform: (&self.path_application).join("platform/macos"),

                id: "macos".to_string(),
                id_symbol: "macos".to_string(),

                configuration: ConfigPlatformConfiguration::Macos(
                    ConfigPlatformMacos {
                        bundle_id: "com.example.unknown".to_string(),
                        namespace: Some("com.example".to_string()),

                        abis: ["arm64", "x86_64"]
                            .iter().map(|v| v.to_string()).collect(),
                        min_os: "10.13".to_string(),

                        version_code: 1,
                        version_name: "1.0.0".to_string(),

                        category: "public.app-category.utilities".to_string(),
                    },
                ),
            },
        );
    }

    // Verify a platform configuration and add it to the set.
    fn add_platform_from_cargo(
        &mut self,
        platform: &cargo::MdOsiPlatform,
    ) -> Result<(), Error> {
        // The ID is always present. Nothing to normalize here.
        let v_id = &platform.id;
        let v_id_symbol = lib::str::symbolize(v_id);

        // Provide a default path based on the platform ID, if none is
        // specified in the configuration.
        let v_path_platform = self.path_application.as_path().join(
            match platform.path.as_ref() {
                Some(v) => v.clone(),
                None => format!("platform/{}", v_id_symbol),
            },
        );

        // Collect the platform-specific configuration.
        let v_configuration = match platform.configuration.as_ref() {
            None => {
                Err(Error::MissingKey(".platforms.[].<type>"))
            },
            Some(cargo::MdOsiPlatformConfiguration::Android(data_android)) => {
                // Java uses reverse-domain paths for all source files. We
                // really need a namespace for the application. We could
                // use `org.example` or `foo.osiris`, but those might show
                // up in the final APK, so we want to avoid it. The user
                // can still specify those if they desire.
                let v_namespace = data_android.namespace.as_ref()
                    .ok_or(Error::MissingKey(".platforms.[].android.namespace"))?;

                // The application ID identifies the application in the
                // different app stores and must be unique. Any changes to
                // the ID will cause the application to be considered
                // different to the original. Hence, the value should be
                // specified explicitly. If not set, we generate it from
                // the namespace and the base application ID.
                let v_application_id = match data_android.application_id.as_ref() {
                    Some(v) => v.clone(),
                    None => {
                        [v_namespace.as_str(), self.id_symbol.as_str()].join(".")
                    },
                };

                // `min-sdk` specifies the minimum SDK version required.
                // `target-sdk` specifies the SDK the application is designed
                // for, and `compile-sdk` is the SDK version the build-tools
                // used at compile time. The latter does not end up in the
                // artifacts and is purely an input to the build tools. It
                // should match `target-sdk`.
                // If any of the three is given, we can pick the others. Note
                // that they are `min <= target <= compile`.
                let (v_min_sdk, v_target_sdk, v_compile_sdk) =
                    match (
                        data_android.min_sdk,
                        data_android.target_sdk,
                        data_android.compile_sdk,
                    ) {
                        (None, None, None) => {
                            return Err(Error::MissingKey(".platforms.[].android.min-sdk"));
                        },
                        (Some(min), None, None) => {
                            (min, min, min)
                        },
                        (None, Some(tar), None) => {
                            (tar, tar, tar)
                        },
                        (None, None, Some(com)) => {
                            (com, com, com)
                        },
                        (Some(min), Some(tar), None) => {
                            (min, tar, tar)
                        },
                        (Some(min), None, Some(com)) => {
                            (min, com, com)
                        },
                        (None, Some(tar), Some(com)) => {
                            (tar, tar, com)
                        },
                        (Some(min), Some(tar), Some(com)) => {
                            (min, tar, com)
                        },
                    };

                // Let the user select the Android ABIs to build for. If it is
                // not specified, we provide the default set with all ABIs.
                let v_abis = if let Some(v) = data_android.abis.as_ref() {
                    v.clone()
                } else {
                    ["armeabi-v7a", "arm64-v8a", "x86", "x86_64"]
                        .iter().map(|v| v.to_string()).collect()
                };

                // The version-code is a simple positive integer increased for
                // every new build. It allows the app stores to identify the
                // builds and decide which one is the most recent. The code has
                // no other meaning. The version-name is used as user-visible
                // version and purely meant as human-readable identification of
                // the version.
                // We can use `1` and `0.1.0` as safe default values, if not
                // provided.
                let v_version_code = data_android.version_code.unwrap_or(1);
                let v_version_name = data_android.version_name.as_deref()
                    .unwrap_or("0.1.0");

                Ok(
                    ConfigPlatformConfiguration::Android(
                        ConfigPlatformAndroid {
                            application_id: v_application_id,
                            namespace: v_namespace.clone(),

                            compile_sdk: v_compile_sdk,
                            min_sdk: v_min_sdk,
                            target_sdk: v_target_sdk,

                            abis: v_abis,

                            version_code: v_version_code,
                            version_name: v_version_name.to_string(),
                        }
                    )
                )
            },
            Some(cargo::MdOsiPlatformConfiguration::Macos(data_macos)) => {
                // The namespace-value is not stored in a macOS bundle, yet we
                // can use it as default for many other IDs. If not provided,
                // we simply leave it unset.
                let v_namespace = data_macos.namespace.clone();

                // The Bundle-ID is used to uniquely identify bundles. It is
                // also used to register applications on the apple servers and
                // to create provisioning profiles. We must allow users to
                // supply it verbatim. If they don't, we use a default based
                // on the namespace or application-ID.
                let v_bundle_id = data_macos.bundle_id.clone()
                    .unwrap_or_else(|| {
                        v_namespace.as_ref()
                            .map(|v| format!("{}.{}", v, &self.id_symbol))
                            .unwrap_or(self.id_symbol.clone())
                    });

                // Let the user select the macOS ABIs to build for. If it is
                // not specified, we provide the default set with all ABIs.
                let v_abis = if let Some(v) = data_macos.abis.as_ref() {
                    v.clone()
                } else {
                    ["arm64", "x86_64"]
                        .iter().map(|v| v.to_string()).collect()
                };

                // Allow users to specify the minimum required OS version, but
                // provide a suitable default. We use the oldest non-deprecated
                // version as default. Apple documents this at:
                //
                //   https://developer.apple.com/documentation/packagedescription/supportedplatform/macosversion
                let v_min_os = data_macos.min_os.clone().unwrap_or("10.13".to_string());

                // The version-code is a simple positive integer increased for
                // every new build. It allows the app stores to identify the
                // builds and decide which one is the most recent. The code has
                // no other meaning. The version-name is used as user-visible
                // version and purely meant as human-readable identification of
                // the version.
                // We can use `1` and `1.0.0` as safe default values, if not
                // provided.
                let v_version_code = data_macos.version_code.unwrap_or(1);
                let v_version_name = data_macos.version_name.clone()
                    .unwrap_or_else(|| format!("{}.0.0", v_version_code));

                // The AppStore uses the category information to group apps
                // according to their primary usage. We supply a default value
                // if none was supplied.
                let v_category = data_macos.category.as_deref()
                    .unwrap_or("public.app-category.utilities")
                    .to_string();

                Ok(
                    ConfigPlatformConfiguration::Macos(
                        ConfigPlatformMacos {
                            bundle_id: v_bundle_id,
                            namespace: v_namespace,

                            abis: v_abis,
                            min_os: v_min_os,

                            version_code: v_version_code,
                            version_name: v_version_name,

                            category: v_category,
                        }
                    )
                )
            },
        }?;

        // Create the platform entry.
        let platform = ConfigPlatform {
            path_platform: v_path_platform,
            id: v_id.clone(),
            id_symbol: v_id_symbol,

            configuration: v_configuration,
        };

        // Check for duplicates. We explicitly do this late for more
        // diagnostics on the actual parameters of each platform.
        match self.platforms.contains_key(&platform.id) {
            true => Err(Error::DuplicatePlatform(platform.id)),
            false => {
                self.platforms.insert(platform.id.clone(), platform);
                Ok(())
            }
        }
    }

    /// Create internal configuration from the Cargo metadata of an application
    /// manifest.
    ///
    /// Take the structured configuration from the Cargo manifest and verify
    /// all content. Parse everything into a secondary structure, which is much
    /// easier to deal with, and has defaults filled in.
    ///
    /// The path to the Cargo manifest file must be provided, to allow relative
    /// paths in the configuration to be resolved. If this path is not
    /// absolute, it is anchored in the current working directory at the time
    /// of this function call.
    pub fn from_cargo(
        data: &cargo::Metadata,
        path_manifest: &dyn AsRef<std::path::Path>,
    ) -> Result<Self, Error> {
        // Remember the absolute path to the directory of the configuration.
        // Other relative paths in the configuration are relative to it.
        let v_path_application = misc::absdir(path_manifest);

        // Remember the absolute path to the Cargo target directory that will
        // be used by this invocation.
        let v_path_target = std::path::Path::new(&data.target_directory).to_path_buf();

        // Use the package-name as application name. Derive its ID from
        // it by masking unsupported characters.
        let mut v_name = data.package_name.clone();
        let mut v_id = v_name.clone();
        let mut v_id_symbol = lib::str::symbolize(&v_id);

        // Use empty icon-information as default.
        let mut v_icons = Vec::new();

        let mut config = match data.osiris {
            None => {
                // Create a default configuration from the information in the
                // Cargo manifest.
                Config {
                    path_application: v_path_application,
                    path_manifest: path_manifest.as_ref().into(),
                    path_target: v_path_target,

                    id: v_id,
                    id_symbol: v_id_symbol,
                    name: v_name,

                    icons: v_icons,

                    platforms: BTreeMap::new(),
                    platform_defaults: BTreeMap::new(),
                }
            },
            Some(cargo::MdOsi::V1(ref mdosi)) => {
                // Override the defaults with values from the application
                // configuration, if any.
                if let Some(ref mdosi_application) = mdosi.application {
                    if let Some(ref v) = mdosi_application.id {
                        v_id = v.into();
                        v_id_symbol = lib::str::symbolize(&v_id);
                        v_name = v_id.clone();
                    }

                    if let Some(ref v) = mdosi_application.name {
                        v_name = v.into();
                    }

                    for icon in &mdosi_application.icons {
                        let Some(ref v_path) = icon.path else { continue };
                        let v_scale = icon.scale.unwrap_or(1);
                        let Some(v_size) = icon.size else { continue };

                        v_icons.push(ConfigIcon {
                            path: v_path.clone(),
                            scale: v_scale,
                            size: v_size,
                        });
                    }
                }

                // Create initial configuration with basic data. Further
                // information will be procedurally added to it.
                let mut config = Config {
                    path_application: v_path_application,
                    path_manifest: path_manifest.as_ref().into(),
                    path_target: v_path_target,

                    id: v_id,
                    id_symbol: v_id_symbol,
                    name: v_name,

                    icons: v_icons,

                    platforms: BTreeMap::new(),
                    platform_defaults: BTreeMap::new(),
                };

                // Collect all platform configuration.
                for platform in mdosi.platforms.iter() {
                    config.add_platform_from_cargo(platform)?;
                }

                config
            },
        };

        // Create defaults for all platforms.
        config.add_default_platforms();

        Ok(config)
    }

    /// Find a platform configuration with the given ID, using the platform
    /// defaults as fallback if no explicit configuration is available.
    pub fn platform(
        &self,
        id: &str,
    ) -> Option<&ConfigPlatform> {
        if let Some(v) = self.platforms.get(id) {
            Some(v)
        } else if let Some(v) = self.platform_defaults.get(id) {
            Some(v)
        } else {
            None
        }
    }
}

impl ConfigPlatform {
    /// ## Return Android Configuration
    ///
    /// Return a reference to the embedded android configuration, or `None`,
    /// depending on whether the platform configuration is for Android.
    pub fn android(&self) -> Option<&ConfigPlatformAndroid> {
        if let ConfigPlatformConfiguration::Android(ref v) = self.configuration {
            Some(v)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify a simple configuration without platforms
    #[test]
    fn simple_config() {
        let data = cargo::Metadata {
            android_sets: Vec::new(),
            osiris: Some(cargo::MdOsi::V1(cargo::MdOsiV1 {
                application: Some(cargo::MdOsiApplication {
                    id: Some("ID".into()),
                    name: None,

                    icons: Vec::new(),
                }),
                platforms: Vec::new(),
            })),
            package_id: "foobar (...)".into(),
            package_name: "foobar".into(),
            target_directory: "./target".into(),
        };
        let config = Config::from_cargo(&data, &".").unwrap();

        assert_eq!(config.id, "ID");
    }
}
