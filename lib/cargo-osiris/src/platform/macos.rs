//! # MacOS Platform Support
//!
//! This module implements application bundles for the macOS platform. It
//! supports direct builds via the XCode tools.

use crate::{cargo, config, op};
use std::collections::BTreeMap;

mod codesign;

pub enum ErrorBuild {
    UnsupportedAbi { abi: String },
}

struct Build<'ctx> {
    // Configuration
    pub build_dir: &'ctx std::path::Path,
    pub macos: &'ctx config::ConfigPlatformMacos,
    pub op: &'ctx op::Build<'ctx>,
}

struct Direct<'ctx> {
    // Build context
    pub build: &'ctx Build<'ctx>,

    // Build directories
    pub artifact_dir: std::path::PathBuf,
    pub bundle_dir: std::path::PathBuf,

    // Artifact files
    pub bundle_plist_file: std::path::PathBuf,
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

    pub fn direct(
        &self,
    ) -> Direct {
        let v_artifact_dir = self.build_dir.join("artifacts");
        let v_bundle_dir = self.build_dir.join("package.app");

        let v_bundle_plist_file = v_artifact_dir.join("package.plist");

        Direct {
            build: self,

            artifact_dir: v_artifact_dir,
            bundle_dir: v_bundle_dir,

            bundle_plist_file: v_bundle_plist_file,
        }
    }
}

impl<'ctx> Direct<'ctx> {
    fn prepare_bundle_plist(&self) -> String {
        format!(
            concat!(
                r#"<?xml version="1.0" encoding="UTF-8"?>"#, "\n",
                r#"<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">"#, "\n",
                r#"<plist version="1.0">"#, "\n",
                r#"  <dict>"#, "\n",
                r#"    <key>CFBundleDisplayName</key>"#, "\n",
                r#"    <string>{0}</string>"#, "\n",
                r#"    <key>CFBundleExecutable</key>"#, "\n",
                r#"    <string>{1}</string>"#, "\n",
                r#"    <key>CFBundleIdentifier</key>"#, "\n",
                r#"    <string>{2}</string>"#, "\n",
                r#"    <key>CFBundleName</key>"#, "\n",
                r#"    <string>{3}</string>"#, "\n",
                r#"    <key>CFBundleVersion</key>"#, "\n",
                r#"    <string>{4}</string>"#, "\n",
                r#"    <key>CFBundlePackageType</key>"#, "\n",
                r#"    <string>{5}</string>"#, "\n",
                r#"  </dict>"#, "\n",
                r#"</plist>"#, "\n",
            ),
            self.build.op.config.name,
            "package.bin",
            self.build.op.config.id,
            self.build.op.config.name,
            "0.1.0",
            "APPL",
        )
    }

    fn prepare(&self) -> Result<(), op::BuildError> {
        // Create build directories
        op::mkdir(&self.artifact_dir)?;
        op::mkdir(&self.bundle_dir)?;

        // Emerge configuration files
        op::update_file(
            self.bundle_plist_file.as_path(),
            self.prepare_bundle_plist().as_bytes(),
        )?;

        Ok(())
    }

    fn build_cargo(&self) -> Result<BTreeMap<String, cargo::Build>, op::BuildError> {
        let mut res = BTreeMap::new();

        // Supported ABI keys are documented in `arch(3)`.
        for &abi in &["arm64"] {
            let target = match abi {
                "arm64" => Ok("aarch64-apple-darwin"),
                "x86_64" => Ok("x86_64-apple-darwin"),
                v => Err(ErrorBuild::UnsupportedAbi { abi: v.into() }),
            }?;

            let query = cargo::BuildQuery {
                cargo_arguments: self.build.op.cargo_arguments,
                envs: Vec::new(),
                target: Some(target.into()),
            };

            let build = query.run()?;
            res.insert(abi.into(), build);
        }

        Ok(res)
    }

    fn build_bundle(
        &self,
        cargo_builds: &BTreeMap<String, cargo::Build>,
    ) -> Result<(), op::BuildError> {
        let mut path = self.bundle_dir.clone();

        op::rmdir(&path)?;

        {
            path.push("Contents");
            op::mkdir(&path)?;

            path.push("Info.plist");
            op::copy_file(&self.bundle_plist_file, &path)?;
            path.pop();

            {
                path.push("Frameworks");
                op::mkdir(&path)?;
                path.pop();
            }

            {
                path.push("MacOS");
                op::mkdir(&path)?;
                path.pop();
            }

            {
                path.push("Resources");
                op::mkdir(&path)?;
                path.pop();
            }

            for (_abi, build) in cargo_builds {
                for artifact in &build.artifacts {
                    let from = std::path::Path::new(&artifact.path);
                    let from_file_name = from.file_name().expect("Cargo artifacts must have file-names");

                    // Copy the executable of the main package into `MacOS`
                    // with the expected name. Copy everything else into the
                    // `Frameworks` directory. We cannot use subdirectories,
                    // since that is strongly discouraged by macOS.
                    //
                    // XXX: We should retain the hierarchy from the build,
                    //      otherwise linking will likely fail. However, we
                    //      should then warn if a non-flat hierarchy is used to
                    //      comply with macOS standards.
                    //
                    // XXX: We should either pick a suitable default binary
                    //      name, or just retain the name and set it
                    //      accordingly in `Info.plist`.
                    //
                    // XXX: We should use `lipo` to merge multi-architecture
                    //      artifacts into universal binaries.
                    if artifact.is_executable
                        && artifact.package_id == self.build.op.cargo_metadata.package_id
                    {
                        path.push("MacOS");
                        path.push("package.bin");
                    } else {
                        path.push("Frameworks");
                        path.push(from_file_name);
                    }

                    op::copy_file(from, &path)?;

                    path.pop();
                    path.pop();
                }
            }

            path.pop();
        }

        Ok(())
    }

    pub fn build(&self) -> Result<(), op::BuildError> {
        self.prepare()?;
        let cargo_builds = self.build_cargo()?;
        self.build_bundle(&cargo_builds)?;

        Ok(())
    }
}

pub fn build(
    op: &op::Build,
    macos: &config::ConfigPlatformMacos,
    build_dir: &std::path::Path,
) -> Result<(), op::BuildError> {
    let build = Build::new(
        op,
        macos,
        build_dir,
    );
    let direct = build.direct();

    direct.build()
}

impl core::fmt::Display for ErrorBuild {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        match self {
            ErrorBuild::UnsupportedAbi { abi } => fmt.write_fmt(core::format_args!("Unsupported ABI: {}", abi)),
        }
    }
}
