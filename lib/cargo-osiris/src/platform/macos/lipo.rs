//! # macOS Universal Binaries
//!
//! This module provides access to the macOS Universal Binary archiver. This is
//! provided via the `lipo` command-line tool on macOS, and allows putting
//! multiple architecture artifacts into a single archive.

use crate::op;

/// Combined arguments to a create-query via `lipo`.
pub struct CreateQuery<'ctx, InputList> {
    /// Paths to the input files
    pub input_files: InputList,
    /// Path to the output file
    pub output_file: &'ctx std::path::Path,
}

impl<'ctx, InputList> CreateQuery<'ctx, InputList>
where
    InputList: Clone + Iterator,
    <InputList as Iterator>::Item: AsRef<std::path::Path>,
{
    /// Execute a lipo-create query via the `lipo` utility of the macOS SDK.
    pub fn run(&self) -> Result<(), op::ErrorProcess> {
        let mut cmd = std::process::Command::new("xcrun");

        // Run a `lipo -create` query.
        cmd.arg("lipo");
        cmd.arg("-create");

        // Append the output file.
        cmd.arg("-output");
        cmd.arg(self.output_file);

        // Append all input files, but ensure proper path prefixes.
        for v in self.input_files.clone() {
            cmd.arg(std::path::Path::new(".").join(v.as_ref()));
        }

        cmd.stderr(std::process::Stdio::inherit());

        let output = cmd.output()
            .map_err(|io| op::ErrorProcess::Exec { name: "lipo".into(), io })?;
        if !output.status.success() {
            return Err(op::ErrorProcess::Exit { name: "lipo".into(), code: output.status });
        }

        Ok(())
    }
}
