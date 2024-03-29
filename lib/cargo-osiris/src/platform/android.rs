//! # Android Platform Support
//!
//! This module provides build-system support for the Android platform. It
//! supports direct builds via the Android SDK, or following the official
//! Gradle build system.

use crate::{cargo, config, lib, op};
use std::collections::BTreeMap;

mod apk;
mod dex;
mod flatres;
mod java;
mod keystore;
mod kotlin;
mod sdk;

/// ## Android Platform Build Errors
///
/// This is an extension of `op::BuildError` with all errors specific to
/// building on the Android platform.
#[derive(Debug)]
pub enum BuildError {
    /// Path contains characters that are not supported by the required tools.
    /// This very likely means the path contains colons or semicolons.
    UnsupportedPath(std::path::PathBuf),
    /// Host platform not supported by the Android SDK.
    UnsupportedHost,
    /// Unsupported target ABI for the Android platform.
    UnsupportedAbi(String),
    /// No Android SDK available, `ANDROID_HOME` is not set.
    NoAndroidHome,
    /// No Android SDK available at the selected location.
    NoSdk(std::path::PathBuf),
    /// Invalid Android SDK at the selected location.
    InvalidSdk(std::path::PathBuf),
    /// No Android Java SDK available at the selected location.
    NoJdk(std::path::PathBuf),
    /// Invalid Android Java SDK at the selected location.
    InvalidJdk(std::path::PathBuf),
    /// No Android Kotlin SDK available at the selected location.
    NoKdk(std::path::PathBuf),
    /// Invalid Android Kotlin SDK at the selected location.
    InvalidKdk(std::path::PathBuf),
    /// No NDK available in the selected Android SDK.
    NoNdk,
    /// Invalid NDK with the selected version in the Android SDK.
    InvalidNdk(std::ffi::OsString),
    /// No Build Tools available in the selected Android SDK.
    NoBuildTools,
    /// Invalid Build Tools with the selected version in the Android SDK.
    InvalidBuildTools(std::ffi::OsString),
    /// No platform for the selected API-level available in the selected
    /// Android SDK.
    NoPlatform(u32),
    /// Invalid platform with the selected API-level in the selected
    /// Android SDK.
    InvalidPlatform(u32),
    /// Execution of the Android resource compiler could not commence.
    FlatresExec(std::io::Error),
    /// Android resource compiler failed executing.
    FlatresExit(std::process::ExitStatus),
    /// Execution of the Java compiler could not commence.
    JavacExec(std::io::Error),
    /// Java compiler failed executing.
    JavacExit(std::process::ExitStatus),
    /// Execution of the Kotlin compiler could not commence.
    KotlincExec(std::io::Error),
    /// Kotlin compiler failed executing.
    KotlincExit(std::process::ExitStatus),
    /// Execution of the DEX compiler could not commence.
    DexExec(std::io::Error),
    /// DEX compiler failed executing.
    DexExit(std::process::ExitStatus),
    /// Execution of the Android APK linker could not commence.
    AaptExec(std::io::Error),
    /// Android APK linker failed executing.
    AaptExit(std::process::ExitStatus),
}

struct Build<'ctx> {
    // Configuration
    pub android: &'ctx config::ConfigPlatformAndroid,
    pub build_dir: &'ctx std::path::Path,
    pub op: &'ctx op::Build<'ctx>,

    // Build directories
    pub apk_dir: std::path::PathBuf,
    pub artifact_dir: std::path::PathBuf,
    pub class_dir: std::path::PathBuf,
    pub dex_dir: std::path::PathBuf,
    pub java_dir: std::path::PathBuf,
    pub resource_dir: std::path::PathBuf,

    // Artifact files
    pub apk_aligned_file: std::path::PathBuf,
    pub apk_base_file: std::path::PathBuf,
    pub apk_linked_file: std::path::PathBuf,
    pub apk_signed_file: std::path::PathBuf,
    pub classes_dex_file: std::path::PathBuf,
    pub debug_keystore_file: std::path::PathBuf,
    pub manifest_file: std::path::PathBuf,
}

struct Direct<'ctx> {
    // Build context
    pub build: &'ctx Build<'ctx>,

    // Tools and resources
    pub build_tools: sdk::BuildTools,
    pub jdk: sdk::Jdk,
    pub kdk: sdk::Kdk,
    pub ndk: sdk::Ndk,
    pub platform: std::path::PathBuf,
    pub platform_jar: std::path::PathBuf,
    pub sdk: sdk::Sdk,
}

impl<'ctx> Build<'ctx> {
    fn new(
        op: &'ctx op::Build,
        android: &'ctx config::ConfigPlatformAndroid,
        build_dir: &'ctx std::path::Path,
    ) -> Self {
        // Prepare build directory paths
        let v_apk_dir = build_dir.join("apk");
        let v_artifact_dir = build_dir.join("artifacts");
        let v_class_dir = build_dir.join("classes");
        let v_dex_dir = build_dir.join("dex");
        let v_java_dir = build_dir.join("java");
        let v_resource_dir = build_dir.join("resources");

        // Prepare artifact file paths
        let v_apk_aligned_file = v_artifact_dir.join("package-aligned.apk");
        let v_apk_base_file = v_artifact_dir.join("package-base.apk");
        let v_apk_linked_file = v_artifact_dir.join("package-linked.apk");
        let v_apk_signed_file = v_artifact_dir.join("package-signed.apk");
        let v_classes_dex_file = v_dex_dir.join("classes.dex");
        let v_debug_keystore_file = v_artifact_dir.join("debug.keystore");
        let v_manifest_file = v_artifact_dir.join("AndroidManifest.xml");

        Self {
            android: android,
            build_dir: build_dir,
            op: op,

            apk_dir: v_apk_dir,
            artifact_dir: v_artifact_dir,
            class_dir: v_class_dir,
            dex_dir: v_dex_dir,
            java_dir: v_java_dir,
            resource_dir: v_resource_dir,

            apk_aligned_file: v_apk_aligned_file,
            apk_base_file: v_apk_base_file,
            apk_linked_file: v_apk_linked_file,
            apk_signed_file: v_apk_signed_file,
            classes_dex_file: v_classes_dex_file,
            debug_keystore_file: v_debug_keystore_file,
            manifest_file: v_manifest_file,
        }
    }

    fn generate_manifest(&self) -> String {
        format!(
            concat!(
                r#"<?xml version="1.0" encoding="utf-8"?>"#, "\n",
                r#"<manifest"#, "\n",
                r#"    xmlns:android="http://schemas.android.com/apk/res/android""#, "\n",
                r#"    xmlns:tools="http://schemas.android.com/tools""#, "\n",
                r#"    package="com.example""#, "\n",
                r#">"#, "\n",
                r#"    <application"#, "\n",
                r#"        android:allowBackup="true""#, "\n",
                r#"        android:supportsRtl="true""#, "\n",
                r#"        tools:targetApi="31">"#, "\n",
                r#"        <activity"#, "\n",
                r#"            android:name=".MainActivity""#, "\n",
                r#"            android:exported="true">"#, "\n",
                r#"            <intent-filter>"#, "\n",
                r#"                <action android:name="android.intent.action.MAIN" />"#, "\n",
                r#"                <category android:name="android.intent.category.LAUNCHER" />"#, "\n",
                r#"            </intent-filter>"#, "\n",
                r#"        </activity>"#, "\n",
                r#"    </application>"#, "\n",
                r#"</manifest>"#, "\n",
            ),
        )
    }

    fn prepare(&self) -> Result<(), op::BuildError> {
        // Create build root
        op::mkdir(self.build_dir)?;

        // Create build directories
        op::mkdir(self.apk_dir.as_path())?;
        op::mkdir(self.artifact_dir.as_path())?;
        op::mkdir(self.class_dir.as_path())?;
        op::mkdir(self.dex_dir.as_path())?;
        op::mkdir(self.java_dir.as_path())?;
        op::mkdir(self.resource_dir.as_path())?;

        // Emerge configuration files
        op::update_file(
            self.debug_keystore_file.as_path(),
            &keystore::DEBUG_DATA,
        )?;
        op::update_file(
            self.manifest_file.as_path(),
            self.generate_manifest().as_bytes(),
        )?;

        Ok(())
    }

    fn direct(&self) -> Result<Direct, op::BuildError> {
        let android_home = match std::env::var_os("ANDROID_HOME") {
            None => Err(BuildError::NoAndroidHome),
            Some(v) => Ok(v),
        }?;
        let v_sdk = match sdk::Sdk::new(std::path::Path::new(&android_home)) {
            Ok(v) => Ok::<_, op::BuildError>(v),
            Err(sdk::SdkError::NoSdk(v)) => Err(BuildError::NoSdk(v).into()),
            Err(sdk::SdkError::InvalidSdk(v)) => Err(BuildError::InvalidSdk(v).into()),
            Err(v) => Err(lib::error::Uncaught::box_debug(v).into()),
        }?;
        let v_ndk = match v_sdk.ndk(None) {
            Ok(v) => Ok::<_, op::BuildError>(v),
            Err(sdk::SdkError::NoNdk) => Err(BuildError::NoNdk.into()),
            Err(sdk::SdkError::InvalidNdk(v)) => Err(BuildError::InvalidNdk(v).into()),
            Err(v) => Err(lib::error::Uncaught::box_debug(v).into()),
        }?;
        let v_build_tools = match v_sdk.build_tools(None) {
            Ok(v) => Ok::<_, op::BuildError>(v),
            Err(sdk::SdkError::NoBuildTools) => Err(BuildError::NoBuildTools.into()),
            Err(sdk::SdkError::InvalidBuildTools(v)) => Err(BuildError::InvalidBuildTools(v).into()),
            Err(v) => Err(lib::error::Uncaught::box_debug(v).into()),
        }?;
        let v_platform = match v_sdk.platform(self.android.min_sdk) {
            Ok(v) => Ok::<_, op::BuildError>(v),
            Err(sdk::SdkError::NoPlatform(v)) => Err(BuildError::NoPlatform(v).into()),
            Err(sdk::SdkError::InvalidPlatform(v)) => Err(BuildError::InvalidPlatform(v).into()),
            Err(v) => Err(lib::error::Uncaught::box_debug(v).into()),
        }?;
        let v_platform_jar = v_platform.as_path().join("android.jar");
        let v_jdk = match sdk::Jdk::new(None).map_err(|v| *v) {
            Ok(v) => Ok::<_, op::BuildError>(v),
            Err(sdk::JdkError::NoJdk(v)) => Err(BuildError::NoJdk(v).into()),
            Err(sdk::JdkError::InvalidJdk(v)) => Err(BuildError::InvalidJdk(v).into()),
        }?;
        let v_kdk = match sdk::Kdk::new(None).map_err(|v| *v) {
            Ok(v) => Ok::<_, op::BuildError>(v),
            Err(sdk::KdkError::NoKdk(v)) => Err(BuildError::NoKdk(v).into()),
            Err(sdk::KdkError::InvalidKdk(v)) => Err(BuildError::InvalidKdk(v).into()),
        }?;

        Ok(Direct {
            build: self,

            build_tools: v_build_tools,
            jdk: v_jdk,
            kdk: v_kdk,
            ndk: v_ndk,
            platform: v_platform,
            platform_jar: v_platform_jar,
            sdk: v_sdk,
        })
    }
}

impl<'ctx> Direct<'ctx> {
    fn build_resources(
        &self,
    ) -> Result<(bool, Vec<std::path::PathBuf>), op::BuildError> {
        let mut res_files: BTreeMap::<std::path::PathBuf, std::path::PathBuf>;
        let mut new = false;

        // Collect all resource files to be compiled. We get a list of
        // resource directories. Each of these contains a list of resource
        // type directories, which then each contains resource files. Any
        // stray entries are silently ignored.
        res_files = BTreeMap::new();
        for rdir in self.build.op.cargo_metadata.android_sets.iter()
            .flat_map(|v| v.resource_dirs.iter())
        {
            let sdirs = std::fs::read_dir(rdir).map_err(
                |io| op::ErrorFileSystem::DirectoryTraversal { path: rdir.into(), io },
            )?;
            for sdir_iter in sdirs {
                let sdir_entry = sdir_iter.map_err(
                    |io| op::ErrorFileSystem::DirectoryTraversal { path: rdir.into(), io },
                )?;
                let sdir = &sdir_entry.path();

                if !sdir.is_dir() {
                    continue;
                }

                let tdirs = std::fs::read_dir(sdir).map_err(
                    |io| op::ErrorFileSystem::DirectoryTraversal { path: sdir.into(), io },
                )?;
                for tdir_iter in tdirs {
                    let tdir_entry = tdir_iter.map_err(
                        |io| op::ErrorFileSystem::DirectoryTraversal { path: sdir.into(), io },
                    )?;
                    let tdir = &tdir_entry.path();

                    if !tdir.is_dir() {
                        // Compute the output file name. Note that this cannot
                        // fail here, since its only failure condition is when
                        // an invalid path, or a path without directory is given.
                        // We just iterated a directory, so both must be set.
                        let out = flatres::Query::output_file_name(
                            tdir,
                        ).ok_or_else(
                            || -> op::BuildError {
                                lib::error::Uncaught::box_any(()).into()
                            },
                        )?;

                        res_files.insert(
                            self.build.resource_dir.join(out),
                            tdir.into(),
                        );
                    }
                }
            }
        }

        // For each resource file, check whether the target file exists and is
        // newer than the source. In this case, skip compilation. Otherwise,
        // invoke the Android flat-resource compiler.
        for (to, from) in &res_files {
            let to_mod = to.metadata().ok()
                .and_then(|v| v.modified().ok());
            let from_mod = from.metadata().ok()
                .and_then(|v| v.modified().ok());
            if let (Some(dst), Some(src)) = (to_mod, from_mod) {
                if src < dst {
                    continue;
                }
            }

            let query = flatres::Query {
                build_tools: self.build_tools.clone(),
                output_dir: self.build.resource_dir.clone(),
                resource_file: from.clone(),
            };

            query.run().map_err(|v| -> op::BuildError {
                match v {
                    flatres::Error::Exec(v) => BuildError::FlatresExec(v).into(),
                    flatres::Error::Exit(v) => BuildError::FlatresExit(v).into(),
                    v => lib::error::Uncaught::box_debug(v).into(),
                }
            })?;

            new = true;
        }

        Ok((new, res_files.into_keys().collect()))
    }

    fn build_apk(
        &self,
        resources: &(bool, Vec<std::path::PathBuf>),
    ) -> Result<bool, op::BuildError> {
        let mut link_files = Vec::new();
        link_files.push(self.platform_jar.clone());

        let query = apk::LinkQuery {
            build_tools: self.build_tools.clone(),
            asset_dirs: Vec::new(),
            link_files: link_files,
            manifest_file: self.build.manifest_file.clone(),
            output_file: self.build.apk_base_file.clone(),
            output_java_dir: Some(self.build.java_dir.clone()),
            resource_files: resources.1.clone(),
        };

        query.run().map_err(|v| -> op::BuildError {
            match v {
                apk::LinkError::Exec(v) => BuildError::FlatresExec(v).into(),
                apk::LinkError::Exit(v) => BuildError::FlatresExit(v).into(),
            }
        })?;

        Ok(true)
    }

    fn build_java(
        &self,
    ) -> Result<bool, op::BuildError> {
        let mut sources = Vec::new();

        // Collect all Java sources. This includes all Java sources specified
        // by the package dependencies, but also the build-generated Java files
        // like the Android `R` class.
        // Since other JVM-languages might use the same source-tree, filter
        // files by their `*.java` extension.

        sources.append(&mut op::lsrdir(self.build.java_dir.as_path())?);

        for set in &self.build.op.cargo_metadata.android_sets {
            for dir in &set.java_dirs {
                sources.append(&mut op::lsrdir(dir.as_path())?);
            }
        }

        sources.retain(|v| v.extension() == Some(std::ffi::OsStr::new("java")));

        if sources.is_empty() {
            return Ok(false);
        }

        // Run the Java compiler. We never try to optimize this and check
        // whether files are up-to-date. Java has a non-trivial source->target
        // file correlation, which is out of scope for us. Just let the
        // compiler deal with it.

        let query = java::Query {
            class_paths: &[&self.platform_jar, &self.build.class_dir],
            jdk: &self.jdk,
            output_dir: &self.build.class_dir,
            source_files: &sources,
        };

        query.run().map_err(|v| -> op::BuildError {
            match v {
                java::Error::UnsupportedPath(v) => BuildError::UnsupportedPath(v).into(),
                java::Error::Exec(v) => BuildError::JavacExec(v).into(),
                java::Error::Exit(v) => BuildError::JavacExit(v).into(),
            }
        })?;

        Ok(true)
    }

    fn build_kotlin(
        &self,
    ) -> Result<bool, op::BuildError> {
        let mut sources = Vec::new();

        // Collect all Kotlin sources. This includes all sources specified by
        // the package dependencies. Note that several JVM-languages might
        // share a source tree, so we filter by the `*.kt` file extension.

        for set in &self.build.op.cargo_metadata.android_sets {
            for dir in &set.kotlin_dirs {
                sources.append(&mut op::lsrdir(dir.as_path())?);
            }
        }
        sources.retain(|v| v.extension() == Some(std::ffi::OsStr::new("kt")));

        if sources.is_empty() {
            return Ok(false);
        }

        // Invoke the Kotlin compiler. Similar to the Java compiler, the
        // source->target file correlation is hard to predict without parsing
        // Java code. Hence, never try to perform timestamp checks and instead
        // let the compiler deal with incremental compilation.

        let query = kotlin::Query {
            class_paths: &[&self.platform_jar, &self.build.class_dir],
            kdk: &self.kdk,
            output_dir: &self.build.class_dir,
            source_files: &sources,
        };

        query.run().map_err(|v| -> op::BuildError {
            match v {
                kotlin::Error::UnsupportedPath(v) => BuildError::UnsupportedPath(v).into(),
                kotlin::Error::Exec(v) => BuildError::KotlincExec(v).into(),
                kotlin::Error::Exit(v) => BuildError::KotlincExit(v).into(),
            }
        })?;

        Ok(true)
    }

    fn build_dex(
        &self,
    ) -> Result<bool, op::BuildError> {
        let mut sources = op::lsrdir(self.build.class_dir.as_path())?;
        sources.retain(|v| v.extension() == Some(std::ffi::OsStr::new("class")));

        if sources.is_empty() {
            return Ok(false);
        }

        let query = dex::Query {
            api: Some(self.build.android.min_sdk),
            build_tools: &self.build_tools,
            class_paths: &Vec::<std::path::PathBuf>::new(),
            debug: false,
            libs: &[&self.platform_jar],
            output_dir: &self.build.dex_dir,
            source_files: &sources,
        };

        query.run().map_err(|v| -> op::BuildError {
            match v {
                dex::Error::Exec(v) => BuildError::DexExec(v).into(),
                dex::Error::Exit(v) => BuildError::DexExit(v).into(),
            }
        })?;

        Ok(true)
    }

    fn build_cargo(
        &self,
    ) -> Result<(bool, BTreeMap<String, cargo::Build>), op::BuildError> {
        let mut res = BTreeMap::new();

        // Android SDKs ship prebuilt toolchains for `x86_64` on linux, macos
        // and windows. However, you can likely call it from `x86` (given the
        // OS runs in 64-bit mode), or on macos via emulators.
        //
        // XXX: The target platform of the calling binary does not have to
        //      match the host-os, nor does it prevent emulators from running
        //      foreign-platform SDKs. We should support selecting the host-os
        //      via cmdline.
        let host = if cfg!(
            all(
                target_os = "linux",
                any(
                    target_arch = "x86",
                    target_arch = "x86_64",
                ),
            ),
        ) {
            "linux-x86_64"
        } else if cfg!(
            all(
                target_os = "macos",
                any(
                    target_arch = "aarch64",
                    target_arch = "x86",
                    target_arch = "x86_64",
                ),
            ),
        ) {
            "darwin-x86_64"
        } else if cfg!(
            all(
                target_os = "windows",
                any(
                    target_arch = "x86",
                    target_arch = "x86_64",
                ),
            ),
        ) {
            "windows-x86_64"
        } else {
            return Err(BuildError::UnsupportedHost.into());
        };

        for abi in &self.build.android.abis {
            let (target, linker_env, linker_prefix) = match abi.as_str() {
                "armeabi-v7a" => Ok((
                    "armv7-linux-androideabi",
                    "CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER",
                    "armv7a-linux-androideabi",
                )),
                "arm64-v8a" => Ok((
                    "aarch64-linux-android",
                    "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
                    "aarch64-linux-android",
                )),
                "x86" => Ok((
                    "i686-linux-android",
                    "CARGO_TARGET_I686_LINUX_ANDROID_LINKER",
                    "i686-linux-android",
                )),
                "x86_64" => Ok((
                    "x86_64-linux-android",
                    "CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER",
                    "x86_64-linux-android",
                )),
                v => Err(BuildError::UnsupportedAbi(v.into())),
            }?;
            let linker_bin = format!(
                "toolchains/llvm/prebuilt/{}/bin/{}{}-clang",
                host,
                linker_prefix,
                self.build.android.min_sdk,
            );
            let linker_path = self.ndk.root().join(linker_bin);

            let query = cargo::BuildQuery {
                cargo_arguments: self.build.op.cargo_arguments,
                cfgs: Vec::new(),
                crate_type: Some("cdylib".into()),
                envs: vec![(linker_env.into(), linker_path.into())],
                target: Some(target.into()),
            };

            let build = query.run().map_err(
                |v| -> op::BuildError { v.into() },
            )?;

            res.insert(abi.into(), build);
        }

        Ok((true, res))
    }

    fn link_apk(
        &self,
        bins: &(bool, BTreeMap<String, cargo::Build>),
    ) -> Result<bool, op::BuildError> {
        let mut path: std::path::PathBuf = self.build.apk_dir.clone();
        let mut add = Vec::new();

        // Copy the intermediate APK to avoid operating on intermediates
        // multiple times, and thus modifying timestamps needlessly.
        op::copy_file(&self.build.apk_base_file, &self.build.apk_linked_file)?;

        // Assemble the APK directory. Since `aapt` retains paths verbatim
        // in the APK, we need to assemble a directory with the exact
        // contents we want in the APK. Unfortunately, this means copying
        // our artifacts to the desired sub-paths in the APK assembly
        // directory.

        path.push("classes.dex");
        add.push("classes.dex".into());
        op::copy_file(&self.build.classes_dex_file, path.as_path())?;
        path.pop();

        path.push("lib");
        op::mkdir(path.as_path())?;
        for (abi, set) in &bins.1 {
            path.push(abi);
            op::mkdir(path.as_path())?;
            for v in &set.artifacts {
                let file_name = std::path::Path::new(&v.path)
                    .file_name()
                    .expect("Cargo artifact has no file-name");

                let mut lib_path = std::ffi::OsString::new();
                lib_path.push(format!("lib/{}/", abi));
                lib_path.push(file_name);

                path.push(file_name);
                add.push(lib_path.into());
                op::copy_file(std::path::Path::new(&v.path), path.as_path())?;
                path.pop();
            }
            path.pop();
        }
        path.pop();

        // Now invoke the `aapt` tools to alter and thus link the final
        // APK. Preferably, we would just invoke a standard ZIP tool, but
        // they are not packaged with the Android SDK, and thus would mean
        // we have another build-time dependency.
        //
        // XXX: Ideally, we would ship, or depend, on a simple ZIP archive
        //      builder in pure Rust, and thus avoid all this dance.

        let query = apk::AlterQuery {
            base_dir: Some(self.build.apk_dir.clone()),
            build_tools: self.build_tools.clone(),
            add_files: add,
            apk_file: self.build.apk_linked_file.clone(),
        };

        query.run().map_err(|v| -> op::BuildError {
            match v {
                apk::AlterError::Exec(v) => BuildError::AaptExec(v).into(),
                apk::AlterError::Exit(v) => BuildError::AaptExit(v).into(),
            }
        })?;

        // Since APKs are normal zip-files, and those have no alignment
        // restrictions, we have to align the file explicitly to ensure
        // Android can run it directly.

        let query = apk::AlignQuery {
            build_tools: self.build_tools.clone(),
            input_file: self.build.apk_linked_file.clone(),
            output_file: self.build.apk_aligned_file.clone(),
        };

        query.run().map_err(|v| -> op::BuildError {
            match v {
                apk::AlignError::Exec(v) => op::BuildError::Exec("zipalign".into(), v),
                apk::AlignError::Exit(v) => op::BuildError::Exit("zipalign".into(), v),
            }
        })?;

        // As last step sign the APK using the debug-keystore. Android requires
        // APKs to be signed (and uses key-information for optional process
        // sharing). Hence, we must sign APKs even during development. For
        // final production, online signing on the Android Store servers is
        // mandatory, so no such support is currently provided here. This can
        // be extended in the future, if offline signing becomes a thing again.

        let query = apk::SignQuery {
            build_tools: self.build_tools.clone(),
            input_file: self.build.apk_aligned_file.clone(),
            keystore: self.build.debug_keystore_file.clone(),
            keystore_key_alias: Some(keystore::DEBUG_KEY_ALIAS.into()),
            keystore_phrase: Some(keystore::DEBUG_PHRASE.into()),
            key_phrase: Some(keystore::DEBUG_KEY_PHRASE.into()),
            output_file: self.build.apk_signed_file.clone(),
        };

        query.run().map_err(|v| -> op::BuildError {
            match v {
                apk::SignError::Exec(v) => op::BuildError::Exec("apksigner".into(), v),
                apk::SignError::Exit(v) => op::BuildError::Exit("apksigner".into(), v),
            }
        })?;

        Ok(true)
    }
}

fn build_direct(
    build: &Build,
) -> Result<(), op::BuildError> {
    let direct = build.direct()?;

    eprintln!("Compile Android resources..");
    let res = direct.build_resources()?;

    eprintln!("Build Android APK..");
    direct.build_apk(&res)?;

    eprintln!("Compile Android Java sources..");
    direct.build_java()?;

    eprintln!("Compile Android Kotlin sources..");
    direct.build_kotlin()?;

    eprintln!("Build DEX files..");
    direct.build_dex()?;

    eprintln!("Build Cargo package..");
    let bins = direct.build_cargo()?;

    eprintln!("Link APK..");
    direct.link_apk(&bins)?;

    Ok(())
}

pub fn build(
    op: &op::Build,
    android: &config::ConfigPlatformAndroid,
    build_dir: &std::path::Path,
) -> Result<(), op::BuildError> {
    let build = Build::new(
        op,
        android,
        build_dir,
    );

    build.prepare()?;
    build_direct(&build)?;

    Ok(())
}

impl core::fmt::Display for BuildError {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        match self {
            BuildError::UnsupportedPath(v) => fmt.write_fmt(core::format_args!("Path contains characters not supported by the Android SDK: {}", v.to_string_lossy())),
            BuildError::UnsupportedHost => fmt.write_fmt(core::format_args!("Host platform not supported by the Android SDK")),
            BuildError::UnsupportedAbi(v) => fmt.write_fmt(core::format_args!("ABI not supported by the Android SDK: {}", v)),
            BuildError::NoAndroidHome => fmt.write_fmt(core::format_args!("No Android SDK available (`ANDROID_HOME` is not set)")),
            BuildError::NoSdk(v) => fmt.write_fmt(core::format_args!("No Android SDK at: {}", v.to_string_lossy())),
            BuildError::InvalidSdk(v) => fmt.write_fmt(core::format_args!("Invalid Android SDK at: {}", v.to_string_lossy())),
            BuildError::NoJdk(v) => fmt.write_fmt(core::format_args!("No Java SDK at: {}", v.to_string_lossy())),
            BuildError::InvalidJdk(v) => fmt.write_fmt(core::format_args!("Invalid Java SDK at: {}", v.to_string_lossy())),
            BuildError::NoKdk(v) => fmt.write_fmt(core::format_args!("No Kotlin SDK at: {}", v.to_string_lossy())),
            BuildError::InvalidKdk(v) => fmt.write_fmt(core::format_args!("Invalid Kotlin SDK at: {}", v.to_string_lossy())),
            BuildError::NoNdk => fmt.write_fmt(core::format_args!("No NDK in the Android SDK")),
            BuildError::InvalidNdk(v) => fmt.write_fmt(core::format_args!("Invalid Android NDK at: {}", v.to_string_lossy())),
            BuildError::NoBuildTools => fmt.write_fmt(core::format_args!("No build-tools in the Android SDK")),
            BuildError::InvalidBuildTools(v) => fmt.write_fmt(core::format_args!("Invalid Android build-tools at: {}", v.to_string_lossy())),
            BuildError::NoPlatform(v) => fmt.write_fmt(core::format_args!("No platform in the Android SDK for API-level: {}", v)),
            BuildError::InvalidPlatform(v) => fmt.write_fmt(core::format_args!("Invalid Android platform for API-level: {}", v)),
            BuildError::FlatresExec(e) => fmt.write_fmt(core::format_args!("Flatres compiler could not commence: {}", e)),
            BuildError::FlatresExit(e) => fmt.write_fmt(core::format_args!("Flatres compiler failed: {}", e)),
            BuildError::JavacExec(e) => fmt.write_fmt(core::format_args!("Java compiler could not commence: {}", e)),
            BuildError::JavacExit(e) => fmt.write_fmt(core::format_args!("Java compiler failed: {}", e)),
            BuildError::KotlincExec(e) => fmt.write_fmt(core::format_args!("Kotlin compiler could not commence: {}", e)),
            BuildError::KotlincExit(e) => fmt.write_fmt(core::format_args!("Kotlin compiler failed: {}", e)),
            BuildError::DexExec(e) => fmt.write_fmt(core::format_args!("DEX compiler could not commence: {}", e)),
            BuildError::DexExit(e) => fmt.write_fmt(core::format_args!("DEX compiler failed: {}", e)),
            BuildError::AaptExec(e) => fmt.write_fmt(core::format_args!("APT linker could not commence: {}", e)),
            BuildError::AaptExit(e) => fmt.write_fmt(core::format_args!("APT linker failed: {}", e)),
        }
    }
}
