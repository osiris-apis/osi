//! # MacOS Platform Support
//!
//! This module implements application bundles for the macOS platform. It
//! supports direct builds via the XCode tools.

use crate::{cargo, config, misc, op};
use std::collections::BTreeMap;

mod actool;
mod codesign;
mod lipo;
mod plistbuddy;
mod productbuild;

pub enum ErrorBuild {
    /// Unsupported target ABI for the macOS platform.
    UnsupportedAbi { abi: String },
    /// Path contains characters that are not supported by the required tools.
    /// This very likely means the path contains non-Unicode characters.
    UnsupportedPath { path: std::path::PathBuf },
    /// Cannot find any resource at the specified directory.
    NoFile { path: std::path::PathBuf },
}

struct ArchivePkg<'ctx> {
    // Configuration
    pub archive_dir: &'ctx std::path::Path,
    pub macos_pkg: &'ctx config::ConfigArchiveMacosPkg,
    pub op: &'ctx op::Archive<'ctx>,
    pub platform_dir: &'ctx std::path::Path,

    // Build directories
    pub artifact_dir: std::path::PathBuf,
    pub bundle_dir: std::path::PathBuf,
    pub platform_bundle_dir: std::path::PathBuf,

    // Artifact files
    pub bundle_entitlements_file: std::path::PathBuf,
    pub pkg_file: std::path::PathBuf,
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
    pub xcassets_dir: std::path::PathBuf,
    pub xcassets_accentcolor_dir: std::path::PathBuf,
    pub xcassets_appicon_dir: std::path::PathBuf,

    // Artifact files
    pub bundle_plist_file: std::path::PathBuf,
    pub bundle_pkginfo_file: std::path::PathBuf,
    pub xcassets_contents_file: std::path::PathBuf,
    pub xcassets_contents_accentcolor_file: std::path::PathBuf,
    pub xcassets_contents_appicon_file: std::path::PathBuf,
    pub xcassets_plist_file: std::path::PathBuf,

    // Pre-compiled attributes
    pub accent_color: Option<&'ctx str>,
    pub app_icon: Option<&'ctx str>,
    pub icons: BTreeMap<(u32, u32), Vec<&'ctx str>>,
}

impl<'ctx> ArchivePkg<'ctx> {
    pub fn new(
        op: &'ctx op::Archive<'ctx>,
        macos_pkg: &'ctx config::ConfigArchiveMacosPkg,
        archive_dir: &'ctx std::path::Path,
        platform_dir: &'ctx std::path::Path,
    ) -> Self {
        let v_artifact_dir = archive_dir.join("artifacts");
        let v_bundle_dir = archive_dir.join(format!("{}.app", op.config.id_symbol));
        let v_platform_bundle_dir = platform_dir.join(format!("{}.app", op.config.id_symbol));

        let v_bundle_entitlements_file = v_artifact_dir.join("bundle.entitlements.plist");
        let v_pkg_file = archive_dir.join(format!("{}.pkg", op.config.id_symbol));

        Self {
            archive_dir: archive_dir,
            macos_pkg: macos_pkg,
            op: op,
            platform_dir: platform_dir,

            artifact_dir: v_artifact_dir,
            bundle_dir: v_bundle_dir,
            platform_bundle_dir: v_platform_bundle_dir,

            bundle_entitlements_file: v_bundle_entitlements_file,
            pkg_file: v_pkg_file,
        }
    }

    fn prepare_bundle_entitlements(&self) -> String {
        let mut acc = String::new();

        acc = format!(
            concat!(
                r#"{}"#,
                r#"<?xml version="1.0" encoding="UTF-8"?>"#, "\n",
                r#"<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">"#, "\n",
                r#"<plist version="1.0">"#, "\n",
                r#"  <dict>"#, "\n",
            ),
            acc,
        );

        acc = format!(
            concat!(
                r#"{}"#,
                r#"    <key>com.apple.security.app-sandbox</key>"#, "\n",
                r#"    <true/>"#, "\n",
            ),
            acc,
        );

        if let Some(ref v) = self.macos_pkg.app_id {
            acc = format!(
                concat!(
                    r#"{}"#,
                    r#"    <key>com.apple.application-identifier</key>"#, "\n",
                    r#"    <string>{}</string>"#, "\n",
                ),
                acc,
                misc::escape_xml_pcdata(v),
            );
        }

        if let Some(ref v) = self.macos_pkg.team_id {
            acc = format!(
                concat!(
                    r#"{}"#,
                    r#"    <key>com.apple.developer.team-identifier</key>"#, "\n",
                    r#"    <string>{}</string>"#, "\n",
                ),
                acc,
                misc::escape_xml_pcdata(v),
            );
        }

        acc = format!(
            concat!(
                r#"{}"#,
                r#"  </dict>"#, "\n",
                r#"</plist>"#, "\n",
            ),
            acc,
        );

        acc
    }

    fn prepare(&self) -> Result<(), op::ArchiveError> {
        // Delete previous artifacts if re-use is not possible.
        op::rmdir(&self.bundle_dir)?;

        // Create build directories
        op::mkdir(&self.artifact_dir)?;
        op::mkdir(&self.bundle_dir)?;

        // Emerge configuration files
        op::update_file(
            self.bundle_entitlements_file.as_path(),
            self.prepare_bundle_entitlements().as_bytes(),
        )?;

        Ok(())
    }

    fn import(&self) -> Result<(), op::ArchiveError> {
        let mut cmd = std::process::Command::new("xcrun");

        // The source directory must end in a slash to ensure its contents
        // are copied rather than the directory itself. `Path` cannot have
        // trailing slashes, so we go the `OsString` route.
        let mut from = self.platform_bundle_dir
            .as_os_str()
            .to_owned();
        from.push("/");

        let mut to = self.bundle_dir.clone();

        cmd.arg("cp");
        cmd.arg("-r");
        cmd.arg("--");
        cmd.arg(&from);
        cmd.arg(&to);

        cmd.stderr(std::process::Stdio::inherit());
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::inherit());

        let status = cmd.status()
            .map_err(|io| op::ErrorProcess::Exec { name: "cp".into(), io })?;
        if !status.success() {
            return Err(op::ErrorProcess::Exit { name: "cp".into(), code: status }.into());
        }

        if let Some(ref provfile) = self.macos_pkg.provision_file {
            to.push("Contents");
            op::mkdir(&to)?;

            to.push("embedded.provisionprofile");
            op::copy_file(provfile, &to)?;
            to.pop();

            to.pop();
        }

        Ok(())
    }

    fn codesign(&self) -> Result<(), op::ArchiveError> {
        let Some(ref sign_identity) = self.macos_pkg.codesign_identity else {
            return Ok(());
        };

        codesign::SignQuery {
            entitlements: Some(&self.bundle_entitlements_file),
            force: false,
            identity: &sign_identity,
            options: Some([
                codesign::SignOption::Runtime,
            ].iter().copied()),
            paths: [
                &self.bundle_dir
            ].iter(),
            requirements: None,
            timestamp: Some(true),
        }.run()?;

        Ok(())
    }

    fn productbuild(&self) -> Result<(), op::ArchiveError> {
        productbuild::BuildQuery {
            components: [
                (self.bundle_dir.as_path(), std::path::Path::new("/Applications")),
            ].iter(),
            identity: self.macos_pkg.pkgsign_identity.as_deref(),
            output_file: &self.pkg_file,
        }.run()?;

        Ok(())
    }

    pub fn run(&self) -> Result<(), op::ArchiveError> {
        self.prepare()?;
        self.import()?;
        self.codesign()?;
        self.productbuild()?;

        Ok(())
    }
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

    // Prepare the macOS collection of application icons from the generic
    // icon information in the configuration.
    //
    // This will create a lookup tree from scale+size to icon-path, allowing
    // better evaluation of icons for macOS asset catalogs.
    fn collect_icons(&self) -> BTreeMap<(u32, u32), Vec<&'ctx str>> {
        let mut icons = BTreeMap::<(u32, u32), Vec<&'ctx str>>::new();

        for icon in &self.op.config.icons {
            icons.entry((icon.scale, icon.size)).or_default().push(&icon.path);
        }

        icons
    }

    pub fn direct(
        &self,
    ) -> Direct {
        let v_artifact_dir = self.build_dir.join("artifacts");
        let v_bundle_dir = self.build_dir.join(format!("{}.app", self.op.config.id_symbol));
        let v_xcassets_dir = v_artifact_dir.join("Assets.xcassets");
        let v_xcassets_accentcolor_dir = v_xcassets_dir.join("AccentColor.colorset");
        let v_xcassets_appicon_dir = v_xcassets_dir.join("AppIcon.appiconset");

        let v_bundle_plist_file = v_artifact_dir.join("package.plist");
        let v_bundle_pkginfo_file = v_artifact_dir.join("PkgInfo");
        let v_xcassets_contents_file = v_xcassets_dir.join("Contents.json");
        let v_xcassets_contents_accentcolor_file = v_xcassets_accentcolor_dir.join("Contents.json");
        let v_xcassets_contents_appicon_file = v_xcassets_appicon_dir.join("Contents.json");
        let v_xcassets_plist_file = v_artifact_dir.join("xcassets.plist");

        let v_accent_color = Some("AccentColor");
        let v_icons = self.collect_icons();
        let v_app_icon = (!v_icons.is_empty()).then(|| "AppIcon");

        Direct {
            build: self,

            artifact_dir: v_artifact_dir,
            bundle_dir: v_bundle_dir,
            xcassets_dir: v_xcassets_dir,
            xcassets_accentcolor_dir: v_xcassets_accentcolor_dir,
            xcassets_appicon_dir: v_xcassets_appicon_dir,

            bundle_plist_file: v_bundle_plist_file,
            bundle_pkginfo_file: v_bundle_pkginfo_file,
            xcassets_contents_file: v_xcassets_contents_file,
            xcassets_contents_accentcolor_file: v_xcassets_contents_accentcolor_file,
            xcassets_contents_appicon_file: v_xcassets_contents_appicon_file,
            xcassets_plist_file: v_xcassets_plist_file,

            accent_color: v_accent_color,
            app_icon: v_app_icon,
            icons: v_icons,
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
                r#"    <key>CFBundleSignature</key>"#, "\n",
                r#"    <string>????</string>"#, "\n",
                "\n",
                r#"    <key>LSApplicationCategoryType</key>"#, "\n",
                r#"    <string>{}</string>"#, "\n",
                r#"    <key>LSMinimumSystemVersion</key>"#, "\n",
                r#"    <string>{}</string>"#, "\n",
                r#"  </dict>"#, "\n",
                r#"</plist>"#, "\n",
            ),
            // XXX: Properly escape the values.
            &self.build.op.config.name,
            &self.build.macos.bundle_id,
            &self.build.op.config.id_symbol,
            &self.build.macos.version_name,
            &self.build.macos.version_code,
            &self.build.op.config.id_symbol,
            &self.build.macos.category,
            &self.build.macos.min_os,
        )
    }

    fn prepare_bundle_pkginfo(&self) -> String {
        "APPL????".into()
    }

    fn prepare_xcassets_contents(&self) -> String {
        format!(
            concat!(
                r#"{{"#, "\n",
                r#"  "info": {{"#, "\n",
                r#"    "author": "{}","#, "\n",
                r#"    "version": {}"#, "\n",
                r#"  }}"#, "\n",
                r#"}}"#, "\n",
            ),
            "xcode",
            1,
        )
    }

    fn prepare_xcassets_contents_accentcolor(&self) -> String {
        format!(
            concat!(
                r#"{{"#, "\n",
                r#"  "colors": ["#, "\n",
                r#"    {{"#, "\n",
                r#"      "idiom": "{}""#, "\n",
                r#"    }}"#, "\n",
                r#"  ],"#, "\n",
                r#"  "info": {{"#, "\n",
                r#"    "author": "{}","#, "\n",
                r#"    "version": {}"#, "\n",
                r#"  }}"#, "\n",
                r#"}}"#, "\n",
            ),
            "universal",
            "xcode",
            1,
        )
    }

    fn prepare_xcassets_contents_appicon(&self) -> Result<String, op::BuildError> {
        let mut json = String::new();
        let mut first = true;
        let mut keep = |filename: &str, scale: u32, size: u32| {
            let leading = if first {
                first = false;
                "\n"
            } else {
                ", \n"
            };

            json = format!(
                concat!(
                    r#"{}{}"#,
                    r#"    {{"#, "\n",
                    r#"      "filename": "{}","#, "\n",
                    r#"      "idiom": "{}","#, "\n",
                    r#"      "scale": "{}x","#, "\n",
                    r#"      "size": "{}x{}""#, "\n",
                    r#"    }}"#,
                ),
                // XXX: Properly escape the paths.
                json,
                leading,
                filename,
                "mac",
                scale,
                size,
                size,
            );
        };

        for (&(scale, size), paths) in &self.icons {
            let path = paths.first().expect("Application icons must have paths");
            let filename = std::path::Path::new(path).file_name()
                .ok_or_else(|| ErrorBuild::NoFile { path: path.into() })?
                .to_str()
                .ok_or_else(|| ErrorBuild::UnsupportedPath { path: path.into() })?;

            // If we are provided with a `1x`-scaled icon with an even
            // width, we can provide it as a half-width `2x`-scaled icon,
            // if none was provided.
            if scale == 1
                && (size % 2) == 0
                && !self.icons.contains_key(&(2, size / 2))
            {
                keep(filename, 2, size / 2);
            }

            keep(filename, scale, size);
        }

        Ok(format!(
            concat!(
                r#"{{"#, "\n",
                r#"  "images": [{}"#, "\n",
                r#"  ],"#, "\n",
                r#"  "info": {{"#, "\n",
                r#"    "author": "{}","#, "\n",
                r#"    "version": {}"#, "\n",
                r#"  }}"#, "\n",
                r#"}}"#, "\n",
            ),
            json,
            "xcode",
            1,
        ))
    }

    fn prepare(&self) -> Result<(), op::BuildError> {
        // Delete previous artifacts if re-use is not possible.
        op::rmdir(&self.bundle_dir)?;
        op::rmdir(&self.xcassets_dir)?;

        // Create build directories
        op::mkdir(&self.artifact_dir)?;
        op::mkdir(&self.bundle_dir)?;
        op::mkdir(&self.xcassets_dir)?;
        op::mkdir(&self.xcassets_accentcolor_dir)?;
        op::mkdir(&self.xcassets_appicon_dir)?;

        // Emerge configuration files
        op::update_file(
            self.bundle_plist_file.as_path(),
            self.prepare_bundle_plist().as_bytes(),
        )?;
        op::update_file(
            self.bundle_pkginfo_file.as_path(),
            self.prepare_bundle_pkginfo().as_bytes(),
        )?;
        op::update_file(
            self.xcassets_contents_file.as_path(),
            self.prepare_xcassets_contents().as_bytes(),
        )?;
        op::update_file(
            self.xcassets_contents_accentcolor_file.as_path(),
            self.prepare_xcassets_contents_accentcolor().as_bytes(),
        )?;
        op::update_file(
            self.xcassets_contents_appicon_file.as_path(),
            self.prepare_xcassets_contents_appicon()?.as_bytes(),
        )?;

        Ok(())
    }

    fn build_cargo(&self) -> Result<BTreeMap<std::path::PathBuf, Vec<std::path::PathBuf>>, op::BuildError> {
        let mut res: BTreeMap<std::path::PathBuf, Vec<std::path::PathBuf>> = BTreeMap::new();

        let mut keep = |singleton: bool, key: &std::path::Path, value: &cargo::BuildArtifact| {
            let path = std::path::Path::new(&value.path).to_path_buf();
            let entry = res.entry(key.into()).or_default();
            if singleton {
                entry.clear();
            }
            entry.push(path);
        };

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
                cfgs: Vec::new(),
                crate_type: Some("bin".into()),
                envs: Vec::new(),
                target: o_target.map(|v| v.into()),
            };

            let build = query.run()?;

            for artifact in build.artifacts {
                let path = std::path::Path::new(&artifact.path);
                let file_name = path.file_name().expect("Cargo artifacts must have file-names");
                let file_path = std::path::Path::new(file_name);
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
                    keep(true, file_path, &artifact);
                    continue;
                };

                // Strip target-specific sub-directory, if possible.
                let path = match o_target {
                    None => path,
                    Some(v) => match path.strip_prefix(v) {
                        Err(_) => {
                            keep(true, file_path, &artifact);
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
                    keep(true, file_path, &artifact);
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
                    keep(
                        false,
                        &std::path::Path::new("Contents/MacOS")
                            .join(&self.build.op.config.id_symbol),
                        &artifact,
                    );
                } else if artifact.package_id == self.build.op.cargo_metadata.package_id {
                    // XXX: We build with `--crate-type bin`, thus only a
                    //      single binary artifact is produced for the main
                    //      package. We can use this reliably as main
                    //      executable.
                    //
                    //      Unfortunately, Cargo does not report executable
                    //      paths for modified crate-types. Hopefully, this
                    //      will be fixed in the future. PR filed.
                    keep(
                        false,
                        &std::path::Path::new("Contents/MacOS")
                            .join(&self.build.op.config.id_symbol),
                        &artifact,
                    );
                } else if artifact.is_executable {
                    // Place helper executables in `Contents/Helpers` without
                    // any hierarchy. Retain the helper name.
                    keep(
                        false,
                        &std::path::Path::new("Contents/Helpers").join(file_name),
                        &artifact,
                    );
                } else if o_extension.is_some_and(
                    |v| v == "bundle" || v == "dylib" || v == "so"
                ) {
                    // Place linker artifacts into `Contents/Frameworks`
                    // without any hierarchy, but with their name retained.
                    keep(
                        false,
                        &std::path::Path::new("Contents/Frameworks").join(file_name),
                        &artifact,
                    );
                } else {
                    // Place everything else in `Contents/Resources` with the
                    // hierarchy retained.
                    keep(
                        true,
                        &std::path::Path::new("Contents/Resources").join(path),
                        &artifact,
                    );
                }
            }
        }

        Ok(res)
    }

    fn build_bundle_car(
        &self,
        res_dir: &std::path::Path,
        info_plist: &std::path::Path,
    ) -> Result<(), op::BuildError> {
        // Copy the icons into the xcassets directory.
        for (_, icons) in &self.icons {
            let icon = icons.first().expect("Application icons must have paths");
            let from = self.build.op.config.path_application.join(icon);
            let file_name = from.file_name().expect("Icon paths must have file-names");
            let to = self.xcassets_appicon_dir.join(file_name);
            op::copy_file(&from, &to)?;
        }

        actool::CompileQuery {
            accent_color: self.accent_color,
            app_icon: self.app_icon,
            input_dirs: [&self.xcassets_dir].iter(),
            min_os: Some(&self.build.macos.min_os),
            output_dir: res_dir,
            output_info_file: Some(&self.xcassets_plist_file),
            verbose: self.build.op.verbose,
        }.run()?;

        plistbuddy::MergeQuery {
            input_file: &self.xcassets_plist_file,
            plist_file: info_plist,
        }.run()?;

        Ok(())
    }

    fn build_bundle(
        &self,
        cargo_builds: &BTreeMap<std::path::PathBuf, Vec<std::path::PathBuf>>,
    ) -> Result<(), op::BuildError> {
        let mut path = self.bundle_dir.clone();
        let info_plist;

        {
            path.push("Contents");
            op::mkdir(&path)?;

            path.push("Info.plist");
            info_plist = path.clone();
            op::copy_file(&self.bundle_plist_file, &info_plist)?;
            path.pop();

            path.push("PkgInfo");
            op::copy_file(&self.bundle_pkginfo_file, &path)?;
            path.pop();

            path.push("Resources");
            op::mkdir(&path)?;
            self.build_bundle_car(&path, &info_plist)?;
            path.pop();

            path.pop();
        }

        for (dst, artifacts) in cargo_builds {
            let to = path.join(dst);
            if let Some(dir) = to.parent() {
                op::mkdir(dir)?;
            }

            match artifacts.len() {
                0 => {},
                1 => op::copy_file(&artifacts[0], &to)?,
                _ => {
                    lipo::CreateQuery {
                        input_files: artifacts.iter(),
                        output_file: &to,
                    }.run()?;
                },
            }
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

pub fn archive_pkg(
    op: &op::Archive,
    macos_pkg: &config::ConfigArchiveMacosPkg,
    archive_dir: &std::path::Path,
    platform_dir: &std::path::Path,
) -> Result<(), op::ArchiveError> {
    let archive = ArchivePkg::new(
        op,
        macos_pkg,
        archive_dir,
        platform_dir,
    );

    archive.run()
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
            ErrorBuild::UnsupportedPath { path } => fmt.write_fmt(core::format_args!("Unsupported path: {}", path.to_string_lossy())),
            ErrorBuild::NoFile { path } => fmt.write_fmt(core::format_args!("No file at the specified path: {}", path.to_string_lossy())),
        }
    }
}
