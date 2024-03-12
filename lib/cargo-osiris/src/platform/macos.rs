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
        let v_bundle_dir = self.build_dir.join(format!("{}.app", self.op.config.id_symbol));

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
                r#"    <string>{}</string>"#, "\n",
                r#"    <key>CFBundleIdentifier</key>"#, "\n",
                r#"    <string>{}</string>"#, "\n",
                r#"    <key>CFBundleName</key>"#, "\n",
                r#"    <string>{}</string>"#, "\n",
                r#"    <key>CFBundleShortVersionString</key>"#, "\n",
                r#"    <string>{}</string>"#, "\n",
                r#"    <key>CFBundleVersion</key>"#, "\n",
                r#"    <string>{}</string>"#, "\n",
                "\n",
                r#"    <key>CFBundleExecutable</key>"#, "\n",
                r#"    <string>{}</string>"#, "\n",
                r#"    <key>CFBundleSupportedPlatforms</key>"#, "\n",
                r#"    <array>"#, "\n",
                r#"      <string>MacOSX</string>"#, "\n",
                r#"    </array>"#, "\n",
                r#"    <key>CFBundlePackageType</key>"#, "\n",
                r#"    <string>APPL</string>"#, "\n",
                "\n",
                r#"    <key>LSApplicationCategoryType</key>"#, "\n",
                r#"    <string>{}</string>"#, "\n",
                r#"  </dict>"#, "\n",
                r#"</plist>"#, "\n",
            ),
            &self.build.op.config.name,
            &self.build.macos.bundle_id,
            &self.build.op.config.id_symbol,
            &self.build.macos.version_name,
            &self.build.macos.version_code,
            &self.build.op.config.id_symbol,
            &self.build.macos.category,
        )
    }

    fn prepare(&self) -> Result<(), op::BuildError> {
        // Delete previous artifacts if re-use is not possible.
        op::rmdir(&self.bundle_dir)?;

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

    fn build_cargo(&self) -> Result<BTreeMap<std::path::PathBuf, cargo::BuildArtifact>, op::BuildError> {
        let mut res = BTreeMap::new();

        // Supported ABI keys are documented in `arch(3)`.
        for abi in &self.build.macos.abis {
            let o_target = match abi.as_str() {
                "arm64" => Ok(Some("aarch64-apple-darwin")),
                "native" => Ok(None),
                "x86_64" => Ok(Some("x86_64-apple-darwin")),
                v => Err(ErrorBuild::UnsupportedAbi { abi: v.into() }),
            }?;

            let query = cargo::BuildQuery {
                cargo_arguments: self.build.op.cargo_arguments,
                envs: Vec::new(),
                target: o_target.map(|v| v.into()),
            };

            let build = query.run()?;

            for artifact in build.artifacts {
                let path = std::path::Path::new(&artifact.path);
                let file_name = path.file_name().expect("Cargo artifacts must have file-names");
                let o_extension = path.extension();

                // For each artifact, try to find its path relative to the
                // main target directory. We have to retain the paths,
                // otherwise they will likely not be placed correctly for the
                // executable to find.
                //
                // If we cannot figure out the relative path (e.g., the
                // artifact is placed outside the target directory), we copy
                // the file without relative path.
                // Similarly, since macOS bundles do not support nested
                // non-bundle hierarchies for dylibs and executables, we copy
                // those without relative path as well.

                // Strip Cargo target directory, if possible.
                let Ok(path) = path.strip_prefix(
                    self.build.op.config.path_target.as_path(),
                ) else {
                    res.insert(file_name.into(), artifact);
                    continue;
                };

                // Strip target-specific sub-directory, if possible.
                let path = match o_target {
                    None => path,
                    Some(v) => match path.strip_prefix(v) {
                        Err(_) => {
                            res.insert(file_name.into(), artifact);
                            continue;
                        },
                        Ok(v) => v,
                    },
                };

                // Strip profile-specific sub-directory, if possible.
                let Ok(path) = path.strip_prefix(
                    match self.build.op.cargo_arguments.profile.as_deref() {
                        None | Some("dev") => "debug",
                        Some(v) => v,
                    },
                ) else {
                    res.insert(file_name.into(), artifact);
                    continue;
                };

                // Depending on the type of artifact, place it into the correct
                // sub-directory of the bundle. Note that MacOS strongly
                // discourages sub-directory hierarchies for executable code,
                // but it is fine for assets. Hence, we strip hierarchy
                // information for all executable code.
                //
                // XXX: Ideally, the caller would have more control over what
                //      is placed where. Unfortunately, we have not found any
                //      reasonable way to convey this metadata. For now, we
                //      simply enforce the heuristic, but some solution for
                //      the future is required.
                //
                // XXX: Optional components should be placed into
                //      `Contents/PlugIns`. Not sure how to deduce that, yet.
                //
                // XXX: Alternative entry-points and root-level helpers should
                //      go into `Contents/MacOS` instead of `Contents/Helpers`.
                //      Again, unsure how to deduce that, yet.
                //
                // XXX: Lastly, lots of nieche use-cases require more elaborate
                //      hierarchy control (e.g., `Contents/XPCServices`,
                //      `Contents/Libraray/...`).
                if artifact.is_executable
                    && artifact.package_id == self.build.op.cargo_metadata.package_id
                {
                    // Place the main executable in `Contents/MacOS` without
                    // any hierarchy. Use the same name as the bundle.
                    res.insert(
                        std::path::Path::new("Contents/MacOS")
                            .join(&self.build.op.config.id_symbol).into(),
                        artifact,
                    );
                } else if artifact.is_executable {
                    // Place helper executables in `Contents/Helpers` without
                    // any hierarchy. Retain the helper name.
                    res.insert(
                        std::path::Path::new("Contents/Helpers").join(file_name).into(),
                        artifact,
                    );
                } else if o_extension.is_some_and(
                    |v| v == "bundle" || v == "dylib" || v == "so"
                ) {
                    // Place linker artifacts into `Contents/Frameworks`
                    // without any hierarchy, but with their name retained.
                    res.insert(
                        std::path::Path::new("Contents/Frameworks").join(file_name).into(),
                        artifact,
                    );
                } else {
                    // Place everything else in `Contents/Resources` with the
                    // hierarchy retained.
                    res.insert(
                        std::path::Path::new("Contents/Resources").join(path).into(),
                        artifact,
                    );
                }
            }
        }

        Ok(res)
    }

    fn build_bundle(
        &self,
        cargo_builds: &BTreeMap<std::path::PathBuf, cargo::BuildArtifact>,
    ) -> Result<(), op::BuildError> {
        let mut path = self.bundle_dir.clone();

        {
            path.push("Contents");
            op::mkdir(&path)?;

            path.push("Info.plist");
            op::copy_file(&self.bundle_plist_file, &path)?;
            path.pop();

            path.pop();
        }

        for (dst, artifact) in cargo_builds {
            let from = std::path::Path::new(&artifact.path);
            let to = path.join(dst);
            if let Some(dir) = to.parent() {
                op::mkdir(dir)?;
            }
            op::copy_file(from, &to)?;
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
