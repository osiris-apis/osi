//! Configuration
//!
//! This provides structured and verified configuration data for the build
//! system. Unlike the TOML-based module, this module provides all
//! configurations in an easy to use format, with helper functions, and
//! with defaults filled in. This module is purely meant to consume the
//! build system configuration. It is not meant to produce any configuration
//! or reproduce the exact layout of the original input. Use the TOML-based
//! module for this.
//!
//! Most of the configuration mirrors the configuration provided by the TOML
//! parser, but has defaults filled in, in case no value was provided, or
//! refuse validation if a mandatory key was missing. See the TOML parser
//! for documentation on most keys, and how they are to be interpreted.

use crate::{toml, util};
use std::collections::BTreeMap;

/// ## Validation Error
///
/// This represents the errors possible when validating the configuration
/// and parsing it into the in-memory structured layout.
#[derive(Debug)]
pub enum Error {
    /// Specified key is required, but missing
    MissingKey(&'static str),
    /// Duplicate platform IDs
    DuplicatePlatform(String),
}

/// ## Android Configuration
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

/// ## Platform Union
pub enum ConfigPlatformConfiguration {
    Android(ConfigPlatformAndroid),
}

/// ## Platform Configuration
pub struct ConfigPlatform {
    /// Absolute path to the platform root.
    pub path_platform: std::path::PathBuf,

    /// Platform ID sourced from TOML
    pub id: String,

    /// Platform-specific configuration
    pub configuration: ConfigPlatformConfiguration,
}

/// ## Configuration Root
pub struct Config {
    /// Absolute path to the TOML configuration.
    pub path_toml: std::path::PathBuf,
    /// Absolute path to the application root.
    pub path_application: std::path::PathBuf,

    /// Application ID sourced from TOML
    pub id: String,
    /// Symbolized Application ID
    pub id_symbol: String,
    /// Application name sourced from TOML
    pub name: String,
    /// Application package name sourced from TOML
    pub package: Option<String>,

    /// Platform Configurations
    pub platforms: BTreeMap<String, ConfigPlatform>,
}

impl Config {
    // Verify a platform configuration and add it to the set.
    fn add_platform(&mut self, platform: &toml::RawPlatform) -> Result<(), Error> {
        // The ID is always present. Nothing to normalize here.
        let v_id = &platform.id;

        // Provide a default path based on the platform ID, if none is
        // specified in the configuration.
        let v_path_platform = self.path_toml.as_path().join(
            match platform.path.as_ref() {
                Some(v) => v.clone(),
                None => format!("platform/{}", v_id),
            },
        );

        // Collect the platform-specific configuration.
        let v_configuration = match platform.configuration.as_ref() {
            None => {
                Err(Error::MissingKey(".platform.<type>"))
            },
            Some(toml::RawPlatformConfiguration::Android(data_android)) => {
                // Java uses reverse-domain paths for all source files. We
                // really need a namespace for the application. We could
                // use `org.example` or `foo.osiris`, but those might show
                // up in the final APK, so we want to avoid it. The user
                // can still specify those if they desire.
                let v_namespace = data_android.namespace.as_ref()
                    .ok_or(Error::MissingKey(".platform.android.namespace"))?;

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
                            return Err(Error::MissingKey(".platform.android.min-sdk"));
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
        }?;

        // Create the platform entry.
        let platform = ConfigPlatform {
            path_platform: v_path_platform,
            id: v_id.clone(),

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

    /// ## Verify the TOML structured configuration
    ///
    /// Take the TOML-parsed structured configuration and verify all content.
    /// Parse everything into a secondary structure, which is much easier to
    /// deal with, and has defaults filled in.
    ///
    /// The path to the TOML configuration file must be provided, to allow
    /// relative paths in the configuration to be resolved. If this path is
    /// not absolute, it is anchored in the current working directory at the
    /// time of this function call.
    pub fn from_toml(
        data: &toml::Raw,
        path: &dyn AsRef<std::path::Path>,
    ) -> Result<Self, Error> {
        // Remember the absolute path to the directory of the configuration.
        // Other relative paths in the configuration are relative to it.
        let v_path_toml = util::absdir(path);

        // `application.id` is required, so `[application]` must be there.
        let data_application = data.application.as_ref()
            .ok_or(Error::MissingKey(".application.id"))?;

        // They application ID is required. We cannot generate it or create
        // a suitable default. A lot of other symbols depend on it, and we do
        // not provide a filler. The user can do that, if they wish.
        let v_id = data_application.id.as_ref()
            .ok_or(Error::MissingKey(".application.id"))?;
        let v_id_symbol = util::symbolize(v_id);

        // Use the application ID as name if none is given.
        let v_name = data_application.name.as_ref().unwrap_or(&v_id);

        // The default path to the application is the manifest directory.
        let v_path_application = v_path_toml.as_path().join(
            data_application.path.as_deref().unwrap_or("."),
        );

        // The main package name is implied as the default workspace target.
        let v_package = data_application.package.clone();

        // Create initial configuration with basic data. Further information
        // will be procedurally added to it.
        let mut config = Config {
            path_toml: v_path_toml,
            path_application: v_path_application,

            id: v_id.clone(),
            id_symbol: v_id_symbol,
            name: v_name.clone(),
            package: v_package,

            platforms: BTreeMap::new(),
        };

        // Collect all platform configuration.
        for platform in data.platform.iter() {
            config.add_platform(platform)?;
        }

        Ok(config)
    }
}

impl ConfigPlatform {
    /// ## Return Android Configuration
    ///
    /// Return a reference to the embedded android configuration, or `None`,
    /// depending on whether the platform configuration is for Android.
    pub fn android(&self) -> Option<&ConfigPlatformAndroid> {
        #[allow(irrefutable_let_patterns)]
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
        let s = r#"
            version = 1
            [application]
            id = "ID"
        "#;

        let raw = toml::Raw::parse_str(s).unwrap();
        let _ = Config::from_toml(&raw, &".").unwrap();
    }
}
