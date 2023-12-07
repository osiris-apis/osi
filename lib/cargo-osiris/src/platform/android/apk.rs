//! # Android Platform APK Manager
//!
//! Android applications are bundled as `Android Package`, or `APK`. These
//! are effectively zip-archives with fixed components.
//!
//! This module allows creation, modification and inspection of APKs.

use crate::platform::android;

/// ## Link Error
///
/// This is the error-enum of all possible errors raised by this
/// linker abstraction.
#[derive(Debug)]
pub enum LinkError {
    /// Program execution failed with the given error.
    Exec(std::io::Error),
    /// Program exited with a failure condition.
    Exit(std::process::ExitStatus),
}

/// ## APK Link Query
///
/// This represents the parameters to an APK link operation. It is to be
/// filled in by the caller.
pub struct LinkQuery {
    /// Android SDK build tools to use for the link.
    pub build_tools: android::sdk::BuildTools,
    /// Asset directories to bundle.
    pub asset_dirs: Vec<std::path::PathBuf>,
    /// APKs to link against.
    pub link_files: Vec<std::path::PathBuf>,
    /// Android manifest for this APK.
    pub manifest_file: std::path::PathBuf,
    /// Output path for the linked APK.
    pub output_file: std::path::PathBuf,
    /// Output path for the generated java resource classes.
    pub output_java_dir: Option<std::path::PathBuf>,
    /// Resource files to link into the APK.
    pub resource_files: Vec<std::path::PathBuf>,
}

/// ## Alter Error
///
/// This is the error-enum of all possible errors raised by this
/// alteration abstraction.
#[derive(Debug)]
pub enum AlterError {
    /// Program execution failed with the given error.
    Exec(std::io::Error),
    /// Program exited with a failure condition.
    Exit(std::process::ExitStatus),
}

/// ## APK Alteration Query
///
/// This represents the parameters to an APK alteration operation. It is to
/// be filled in by the caller.
pub struct AlterQuery {
    /// Android SDK build tools to use for the link.
    pub build_tools: android::sdk::BuildTools,
    /// Files to add to the APK.
    pub add_files: Vec<std::path::PathBuf>,
    /// APK file to modify.
    pub apk_file: std::path::PathBuf,
}

impl LinkQuery {
    /// ## Run `aapt2` linker
    ///
    /// Run the `aapt2` APK linker, producing an APK for the given input
    /// resources.
    pub fn run(&self) -> Result<(), LinkError> {
        // Set up basic `aapt2 link` command.
        let mut cmd = std::process::Command::new(
            self.build_tools.aapt2()
        );
        cmd.args([
            "link",
        ]);

        // Append asset directories.
        for v in &self.asset_dirs {
            cmd.arg("-A");
            cmd.arg(v);
        }

        // Append linker includes.
        for v in &self.link_files {
            cmd.arg("-I");
            cmd.arg(v);
        }

        // Append manifest path.
        cmd.arg("--manifest");
        cmd.arg(&self.manifest_file);

        // Specify output file.
        cmd.arg("-o");
        cmd.arg(&self.output_file);

        // Specify java output directory.
        if let Some(ref v) = self.output_java_dir {
            cmd.arg("--java");
            cmd.arg(v);
        }

        // Append all input resource files. Ensure that they start with a
        // proper path prefix, since `aapt2` does not support `--` separators.
        for v in &self.resource_files {
            cmd.arg(std::path::Path::new(".").join(v));
        }

        // Always forward diagnostics to the parent error stream, so
        // the user can inspect them.
        cmd.stderr(std::process::Stdio::inherit());

        // Run and verify it exited successfully.
        let output = cmd.output().map_err(|v| LinkError::Exec(v))?;
        if !output.status.success() {
            return Err(LinkError::Exit(output.status));
        }

        // Not interested in the output of the tool.
        drop(output);

        Ok(())
    }
}

impl AlterQuery {
    /// ## Run `aapt` alteration
    ///
    /// Run the `aapt` APK tool to alter an existing APK.
    pub fn run(&self) -> Result<(), AlterError> {
        // Set up basic `aapt add` command.
        let mut cmd = std::process::Command::new(
            self.build_tools.aapt()
        );
        cmd.args([
            "add",
        ]);

        // Append path to the APK, but ensure proper path prefixes.
        cmd.arg(std::path::Path::new(".").join(&self.apk_file));

        // Append all input resource files. Ensure that they start with a
        // proper path prefix, since `aapt` does not support `--` separators.
        for v in &self.add_files {
            cmd.arg(std::path::Path::new(".").join(v));
        }

        // Always forward diagnostics to the parent error stream, so
        // the user can inspect them.
        cmd.stderr(std::process::Stdio::inherit());

        // Run and verify it exited successfully.
        let output = cmd.output().map_err(|v| AlterError::Exec(v))?;
        if !output.status.success() {
            return Err(AlterError::Exit(output.status));
        }

        // Not interested in the output of the tool.
        drop(output);

        Ok(())
    }
}
