//! # Android Platform Support
//!
//! This module provides build-system support for the Android platform. It
//! supports direct builds via the Android SDK, or following the official
//! Gradle build system.

use crate::{cargo, config, lib, op};
use std::collections::BTreeMap;

mod apk;
mod flatres;
mod java;
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
}

struct Build<'ctx> {
    // Configuration
    pub android: &'ctx config::ConfigPlatformAndroid,
    pub build_dir: &'ctx std::path::Path,
    pub config: &'ctx config::Config,
    pub metadata: &'ctx cargo::Metadata,
    pub platform: &'ctx config::ConfigPlatform,

    // Build directories
    pub artifact_dir: std::path::PathBuf,
    pub class_dir: std::path::PathBuf,
    pub java_dir: std::path::PathBuf,
    pub resource_dir: std::path::PathBuf,

    // Artifact files
    pub apk_file: std::path::PathBuf,
    pub manifest_file: std::path::PathBuf,
}

struct Direct<'ctx> {
    // Build context
    pub build: &'ctx Build<'ctx>,

    // Tools and resources
    pub build_tools: sdk::BuildTools,
    pub jdk: sdk::Jdk,
    pub kdk: sdk::Kdk,
    pub platform: std::path::PathBuf,
    pub platform_jar: std::path::PathBuf,
    pub sdk: sdk::Sdk,
}

impl<'ctx> Build<'ctx> {
    fn new(
        config: &'ctx config::Config,
        metadata: &'ctx cargo::Metadata,
        platform: &'ctx config::ConfigPlatform,
        android: &'ctx config::ConfigPlatformAndroid,
        build_dir: &'ctx std::path::Path,
    ) -> Self {
        // Prepare build directory paths
        let v_artifact_dir = build_dir.join("artifacts");
        let v_class_dir = build_dir.join("classes");
        let v_java_dir = build_dir.join("java");
        let v_resource_dir = build_dir.join("resources");

        // Prepare artifact file paths
        let v_apk_file = v_artifact_dir.join("package.apk");
        let v_manifest_file = v_artifact_dir.join("AndroidManifest.xml");

        Self {
            android: android,
            build_dir: build_dir,
            config: config,
            metadata: metadata,
            platform: platform,

            artifact_dir: v_artifact_dir,
            class_dir: v_class_dir,
            java_dir: v_java_dir,
            resource_dir: v_resource_dir,

            apk_file: v_apk_file,
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
        op::mkdir(self.artifact_dir.as_path())?;
        op::mkdir(self.class_dir.as_path())?;
        op::mkdir(self.java_dir.as_path())?;
        op::mkdir(self.resource_dir.as_path())?;

        // Emerge configuration files
        op::update_file(
            self.manifest_file.as_path(),
            &self.generate_manifest(),
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
        let mut fresh = false;

        // Collect all resource files to be compiled. We get a list of
        // resource directories. Each of these contains a list of resource
        // type directories, which then each contains resource files. Any
        // stray entries are silently ignored.
        res_files = BTreeMap::new();
        for rdir in self.build.metadata.android_sets.iter()
            .map(|v| v.resource_dirs.iter())
            .flatten()
        {
            let sdirs = std::fs::read_dir(rdir).map_err(
                |_| op::BuildError::DirectoryTraversal(rdir.into()),
            )?;
            for sdir_iter in sdirs {
                let sdir_entry = sdir_iter.map_err(
                    |_| op::BuildError::DirectoryTraversal(rdir.into()),
                )?;
                let sdir = &sdir_entry.path();

                if !sdir.is_dir() {
                    continue;
                }

                let tdirs = std::fs::read_dir(sdir).map_err(
                    |_| op::BuildError::DirectoryTraversal(sdir.into()),
                )?;
                for tdir_iter in tdirs {
                    let tdir_entry = tdir_iter.map_err(
                        |_| op::BuildError::DirectoryTraversal(sdir.into()),
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
                .map(|v| v.modified().ok())
                .flatten();
            let from_mod = from.metadata().ok()
                .map(|v| v.modified().ok())
                .flatten();
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

            fresh = true;
        }

        Ok((fresh, res_files.into_keys().collect()))
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
            output_file: self.build.apk_file.clone(),
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

        for set in &self.build.metadata.android_sets {
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
            class_paths: &[&self.platform_jar],
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

        for set in &self.build.metadata.android_sets {
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
            class_paths: &[&self.platform_jar],
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

    fn build_cargo(
        &self,
    ) -> Result<bool, op::BuildError> {
        let query = cargo::BuildQuery {
            default_features: true,
            features: Vec::new(),
            profile: None,
            target: None,
            workspace: self.build.config.path_application.clone(),
        };

        let _build = query.run().map_err(
            |v| -> op::BuildError { v.into() },
        )?;

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

    eprintln!("Build Cargo package..");
    direct.build_cargo()?;

    Ok(())
}

pub fn build(
    config: &config::Config,
    metadata: &cargo::Metadata,
    platform: &config::ConfigPlatform,
    android: &config::ConfigPlatformAndroid,
    build_dir: &std::path::Path,
) -> Result<(), op::BuildError> {
    let build = Build::new(
        config,
        metadata,
        platform,
        android,
        build_dir,
    );

    build.prepare()?;
    build_direct(&build)?;

    Ok(())
}
