//! # macOS Asset Catalogs
//!
//! This module provides access to the macOS Asset Catalog infrastructure. This
//! wraps the `actool` command and allows compiling asset catalogs from their
//! source information.

use crate::op;

/// Combined arguments to a compile-query.
pub struct CompileQuery<'ctx, InputList> {
    /// Accent color to select.
    pub accent_color: Option<&'ctx str>,
    /// Application icon to extract
    pub app_icon: Option<&'ctx str>,
    /// Paths to the input directories
    pub input_dirs: InputList,
    /// Minimum deployment target
    pub min_os: Option<&'ctx str>,
    /// Path to the output resource directory
    pub output_dir: &'ctx std::path::Path,
    /// Path to the output info file
    pub output_info_file: Option<&'ctx std::path::Path>,
}

impl<'ctx, InputList> CompileQuery<'ctx, InputList>
where
    InputList: Clone + Iterator,
    <InputList as Iterator>::Item: AsRef<std::path::Path>,
{
    /// Execute an actool-compile query.
    pub fn run(&self) -> Result<(), op::ErrorProcess> {
        let mut cmd = std::process::Command::new("xcrun");

        // Run an `actool --compile` query.
        cmd.arg("actool");
        cmd.arg("--compile");
        cmd.arg(self.output_dir);

        // Select the accent-color.
        if let Some(v) = self.accent_color.as_ref() {
            cmd.arg("--accent-color");
            cmd.arg(v);
        }

        // Select the app-icon.
        if let Some(v) = self.app_icon.as_ref() {
            cmd.arg("--app-icon");
            cmd.arg(v);
        }

        // Enable errors.
        cmd.arg("--errors");

        // Append minimum-deployment-target requirements.
        if let Some(v) = self.min_os.as_ref() {
            cmd.arg("--minimum-deployment-target");
            cmd.arg(format!("{}", v));
        }

        // Enable notices.
        cmd.arg("--notices");

        // Request human-readable output.
        cmd.arg("--output-format");
        cmd.arg("human-readable-text");

        // Request Info.plist output.
        if let Some(v) = self.output_info_file {
            cmd.arg("--output-partial-info-plist");
            cmd.arg(v);
        }

        // Limit the target platforms to macOS.
        cmd.arg("--platform");
        cmd.arg("macosx");

        // Limit the target devices to Macs.
        cmd.arg("--target-device");
        cmd.arg("mac");

        // Enable warnings.
        cmd.arg("--warnings");

        // Append all input files.
        cmd.arg("--");
        for v in self.input_dirs.clone() {
            cmd.arg(v.as_ref());
        }

        cmd.stderr(std::process::Stdio::inherit());

        let output = cmd.output()
            .map_err(|io| op::ErrorProcess::Exec { name: "actool".into(), io })?;
        if !output.status.success() {
            return Err(op::ErrorProcess::Exit { name: "actool".into(), code: output.status });
        }

        Ok(())
    }
}
