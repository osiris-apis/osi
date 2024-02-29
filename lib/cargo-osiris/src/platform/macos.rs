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
    pub macos: &'ctx config::ConfigPlatformMacos,
    pub op: &'ctx op::Build<'ctx>,
}

impl<'ctx> Build<'ctx> {
    pub fn new(
        op: &'ctx op::Build<'ctx>,
        macos: &'ctx config::ConfigPlatformMacos,
        build_dir: &'ctx std::path::Path,
    ) -> Self {
        Self {
            build_dir: build_dir,
            macos: macos,
            op: op,
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
    op: &op::Build,
    macos: &config::ConfigPlatformMacos,
    build_dir: &std::path::Path,
) -> Result<(), op::BuildError> {
    let _build = Build::new(
        op,
        macos,
        build_dir,
    );

    Ok(())
}
