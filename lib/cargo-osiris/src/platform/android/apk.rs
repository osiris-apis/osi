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
    /// Base directory to run the command in (to ensure relative paths are
    /// anchored correctly). Note that paths are retained in the APK, so
    /// very likely relative paths are desired.
    pub base_dir: Option<std::path::PathBuf>,
    /// Android SDK build tools to use for the link.
    pub build_tools: android::sdk::BuildTools,
    /// Files to add to the APK.
    pub add_files: Vec<std::path::PathBuf>,
    /// APK file to modify.
    pub apk_file: std::path::PathBuf,
}

/// ## Align Error
///
/// This is the error-enum of all possible errors raised by this
/// align abstraction.
#[derive(Debug)]
pub enum AlignError {
    /// Program execution failed with the given error.
    Exec(std::io::Error),
    /// Program exited with a failure condition.
    Exit(std::process::ExitStatus),
}

/// ## APK Align Query
///
/// This represents the parameters to an APK align operation. It is to
/// be filled in by the caller.
pub struct AlignQuery {
    /// Android SDK build tools to use for the link.
    pub build_tools: android::sdk::BuildTools,
    /// Input path for the unaligned APK.
    pub input_file: std::path::PathBuf,
    /// Output path for the aligned APK.
    pub output_file: std::path::PathBuf,
}

/// ## Sign Error
///
/// This is the error-enum of all possible errors raised by this
/// sign abstraction.
#[derive(Debug)]
pub enum SignError {
    /// Program execution failed with the given error.
    Exec(std::io::Error),
    /// Program exited with a failure condition.
    Exit(std::process::ExitStatus),
}

/// ## APK Sign Query
///
/// This represents the parameters to an APK sign operation. It is to
/// be filled in by the caller.
pub struct SignQuery {
    /// Android SDK build tools to use for the link.
    pub build_tools: android::sdk::BuildTools,
    /// Input path for the unsigned APK.
    pub input_file: std::path::PathBuf,
    /// Path to the keystore file.
    pub keystore: std::path::PathBuf,
    /// Key alias of the key in the keystore.
    pub keystore_key_alias: Option<String>,
    /// Pass phrase for the keystore.
    pub keystore_phrase: Option<String>,
    /// Pass phrase for the key.
    pub key_phrase: Option<String>,
    /// Output path for the signed APK.
    pub output_file: std::path::PathBuf,
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

        // Append all input resource files. Pass them verbatim. While `--` is
        // not supported as separator, the caller should ensure the paths are
        // provided suitably.
        for v in &self.add_files {
            cmd.arg(v);
        }

        // Set the working directory for `aapt` to allow paths to be specified
        // as relative paths, given that they are retained verbatim in the
        // target archive.
        if let Some(ref v) = self.base_dir {
            cmd.current_dir(v);
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

impl AlignQuery {
    /// ## Run `zipalign`
    ///
    /// Run the `zipalign` APK tool to align an existing APK.
    pub fn run(&self) -> Result<(), AlignError> {
        // Set up basic `zipalign` command.
        let mut cmd = std::process::Command::new(
            self.build_tools.zipalign()
        );
        cmd.args([
            "-f",
            "-p",
            "-v",
            "4",
        ]);

        // Append paths to the APK, but ensure proper path prefixes.
        cmd.arg(std::path::Path::new(".").join(&self.input_file));
        cmd.arg(std::path::Path::new(".").join(&self.output_file));

        // Always forward diagnostics to the parent error stream, so
        // the user can inspect them.
        cmd.stderr(std::process::Stdio::inherit());

        // Run and verify it exited successfully.
        let output = cmd.output().map_err(|v| AlignError::Exec(v))?;
        if !output.status.success() {
            return Err(AlignError::Exit(output.status));
        }

        // Not interested in the output of the tool.
        drop(output);

        Ok(())
    }
}

impl SignQuery {
    /// ## Run `apksigner`
    ///
    /// Run the `apksigner` APK tool to sign an existing APK.
    pub fn run(&self) -> Result<(), SignError> {
        // Set up basic `apksigner` command.
        let mut cmd = std::process::Command::new(
            self.build_tools.apksigner()
        );
        cmd.args([
            "sign",
            "-v",
        ]);

        // Append keystore.
        cmd.arg("--ks");
        cmd.arg(&self.keystore);

        // Append keystore key alias.
        if let Some(ref v) = self.keystore_key_alias {
            cmd.arg("--ks-key-alias");
            cmd.arg(v);
        }

        // Append keystore credentials.
        if let Some(ref v) = self.keystore_phrase {
            cmd.env("CARGO_OSIRIS_ANDROID_KEYSTORE_PHRASE", v);
            cmd.arg("--ks-pass");
            cmd.arg("env:CARGO_OSIRIS_ANDROID_KEYSTORE_PHRASE");
        }

        // Append key credentials.
        if let Some(ref v) = self.key_phrase {
            cmd.env("CARGO_OSIRIS_ANDROID_KEY_PHRASE", v);
            cmd.arg("--key-pass");
            cmd.arg("env:CARGO_OSIRIS_ANDROID_KEY_PHRASE");
        }

        // Append path to the output APK.
        cmd.arg("--out");
        cmd.arg(&self.output_file);

        // Append path to the input APK, but ensure proper path prefixes.
        cmd.arg(std::path::Path::new(".").join(&self.input_file));

        // Always forward diagnostics to the parent error stream, so
        // the user can inspect them.
        cmd.stderr(std::process::Stdio::inherit());

        // Run and verify it exited successfully.
        let output = cmd.output().map_err(|v| SignError::Exec(v))?;
        if !output.status.success() {
            return Err(SignError::Exit(output.status));
        }

        // Not interested in the output of the tool.
        drop(output);

        Ok(())
    }
}
