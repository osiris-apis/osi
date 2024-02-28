//! # MacOS Platform Support
//!
//! This module implements application bundles for the macOS platform. It
//! supports direct builds via the XCode tools.

use crate::{cargo, config, op};

pub enum ErrorBuild {
}

struct Build<'ctx> {
    // Configuration
    pub build_dir: &'ctx std::path::Path,
    pub config: &'ctx config::Config,
    pub macos: &'ctx config::ConfigPlatformMacos,
    pub metadata: &'ctx cargo::Metadata,
    pub platform: &'ctx config::ConfigPlatform,
}

impl<'ctx> Build<'ctx> {
    fn new(
        config: &'ctx config::Config,
        metadata: &'ctx cargo::Metadata,
        platform: &'ctx config::ConfigPlatform,
        macos: &'ctx config::ConfigPlatformMacos,
        build_dir: &'ctx std::path::Path,
    ) -> Self {
        Self {
            build_dir: build_dir,
            config: config,
            macos: macos,
            metadata: metadata,
            platform: platform,
        }
    }
}

impl core::fmt::Display for ErrorBuild {
    fn fmt(&self, _fmt: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        match self {
            _ => todo!(),
        }
    }
}

pub fn build(
    config: &config::Config,
    metadata: &cargo::Metadata,
    platform: &config::ConfigPlatform,
    macos: &config::ConfigPlatformMacos,
    build_dir: &std::path::Path,
) -> Result<(), op::BuildError> {
    let _build = Build::new(
        config,
        metadata,
        platform,
        macos,
        build_dir,
    );

    Ok(())
}
