//! # macOS PlistBuddy
//!
//! The `PlistBuddy` utility is a standard macOS helper to operate on plist
//! files. This module provides structured access to its features.

use crate::{misc, op};

/// Combined arguments to a merge-query.
///
/// Conflicting entries will not be merged, but the original is retained.
pub struct MergeQuery<'ctx> {
    /// Path to the input files to merge into the plist file
    pub input_file: &'ctx std::path::Path,
    /// Path to the plist file to modify
    pub plist_file: &'ctx std::path::Path,
}

impl<'ctx> MergeQuery<'ctx> {
    /// Execute a merge query via the `PlistBuddy` utility of macOS.
    pub fn run(&self) -> Result<(), op::ErrorProcess> {
        let mut cmd = std::process::Command::new("/usr/libexec/PlistBuddy");

        // Assemble the merge query.
        //
        // The path can be escaped in single-quotes with backslashes as
        // escape character.
        let mut qr: std::ffi::OsString = "Merge '".into();
        qr.push(misc::escape_single_quote(self.input_file.as_os_str()));
        qr.push("'");

        // Run a merge query.
        cmd.arg("-x");
        cmd.arg("-c");
        cmd.arg(qr);
        cmd.arg(std::path::Path::new(".").join(self.plist_file));

        cmd.stderr(std::process::Stdio::inherit());

        let output = cmd.output()
            .map_err(|io| op::ErrorProcess::Exec { name: "PlistBuddy".into(), io })?;
        if !output.status.success() {
            return Err(op::ErrorProcess::Exit { name: "PlistBuddy".into(), code: output.status });
        }

        Ok(())
    }
}
