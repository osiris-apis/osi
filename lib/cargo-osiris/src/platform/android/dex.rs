//! # Android Platform Dexer
//!
//! This module provides helpers to compile java byte-code into the DEX format
//! using the Android D8 compiler.

use crate::platform::android;

/// ## Compilation Error
///
/// This is the error-enum of all possible errors raised by this D8
/// compilation abstraction.
#[derive(Debug)]
pub enum Error {
    /// Program execution failed with the given error.
    Exec(std::io::Error),
    /// Program exited with a failure condition.
    Exit(std::process::ExitStatus),
}

/// ## D8 Compiler Query
///
/// This represents the parameters to a D8 compilation. It is to
/// be filled in by the caller.
pub struct Query<'ctx, CpList, LibList, SrcList> {
    /// Minimum API level to build for.
    pub api: Option<u32>,
    /// Build-tools to use for the compilation.
    pub build_tools: &'ctx android::sdk::BuildTools,
    /// Directories and files to make up the class-path.
    pub class_paths: CpList,
    /// Whether to include debug information.
    pub debug: bool,
    /// Libraries to link to.
    pub libs: LibList,
    /// Output directory where to store the DEX files.
    pub output_dir: &'ctx std::path::Path,
    /// Source files to compile.
    pub source_files: SrcList,
}

impl<'ctx, CpList, LibList, SrcList> Query<'ctx, CpList, LibList, SrcList>
where
    CpList: Clone + IntoIterator,
    <CpList as IntoIterator>::Item: AsRef<std::path::Path>,
    LibList: Clone + IntoIterator,
    <LibList as IntoIterator>::Item: AsRef<std::path::Path>,
    SrcList: Clone + IntoIterator,
    <SrcList as IntoIterator>::Item: AsRef<std::path::Path>,
{
    /// ## Run `d8` compiler
    ///
    /// Run the `d8` compiler to compile the specified source files for
    /// the configured Android Platform.
    pub fn run(&self) -> Result<(), Error> {
        // Set up basic `d8` command.
        let mut cmd = std::process::Command::new(
            self.build_tools.d8(),
        );

        // XXX: We should run this as 2-step process via the 
        //      `--file-per-class-file` argument, to get one DEX file
        //      per input class-file. This allows timestamp comparisons
        //      and skipping DEX compilation unless really required.

        // Append class-path entries.
        for v in self.class_paths.clone() {
            cmd.arg("--classpath");
            cmd.arg(v.as_ref());
        }

        // Append debug flag if requested.
        if self.debug {
            cmd.arg("--debug");
        }

        // Append library entries.
        for v in self.libs.clone() {
            cmd.arg("--lib");
            cmd.arg(v.as_ref());
        }

        // Append minimum API level if set.
        if let Some(v) = self.api {
            cmd.arg("--min-api");
            cmd.arg(format!("{}", v));
        }

        // Select a suitable output directory.
        cmd.arg("--output");
        cmd.arg(&self.output_dir);

        // Append release flag as default
        if !self.debug {
            cmd.arg("--release");
        }

        // Append all source paths. We ensure they start with a path indicator,
        // since `d8` does not support `--` separators.
        for v in self.source_files.clone() {
            cmd.arg(std::path::Path::new(".").join(v));
        }

        // Always forward diagnostics to the parent error stream, so
        // the user can inspect them.
        cmd.stderr(std::process::Stdio::inherit());

        // Run and verify it exited successfully.
        let output = cmd.output().map_err(|v| Error::Exec(v))?;
        if !output.status.success() {
            return Err(Error::Exit(output.status));
        }

        // Not interested in the output of the tool.
        drop(output);

        Ok(())
    }
}
