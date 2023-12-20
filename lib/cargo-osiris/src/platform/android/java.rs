//! # Android Platform Java Compilation
//!
//! This module provides helpers to compile java code for the Android
//! Platform.

use crate::platform::android;

/// ## Compilation Error
///
/// This is the error-enum of all possible errors raised by this Java
/// compilation abstraction.
#[derive(Debug)]
pub enum Error {
    /// Unsupported path (likely containing characters that cannot be escaped).
    UnsupportedPath(std::path::PathBuf),
    /// Program execution failed with the given error.
    Exec(std::io::Error),
    /// Program exited with a failure condition.
    Exit(std::process::ExitStatus),
}

/// ## Java Compiler Query
///
/// This represents the parameters to a java compilation. It is to
/// be filled in by the caller.
pub struct Query<'ctx, CpList, SrcList> {
    /// Directories and files to make up the Java class-path.
    pub class_paths: CpList,
    /// JDK to use for the compilation.
    pub jdk: &'ctx android::sdk::Jdk,
    /// Output directory where to store the class files.
    pub output_dir: &'ctx std::path::Path,
    /// Source files to compile.
    pub source_files: SrcList,
}

impl<'ctx, CpList, SrcList> Query<'ctx, CpList, SrcList>
where
    CpList: Clone + IntoIterator,
    <CpList as IntoIterator>::Item: AsRef<std::path::Path>,
    SrcList: Clone + IntoIterator,
    <SrcList as IntoIterator>::Item: AsRef<std::path::Path>,
{
    /// ## Run `javac` compiler
    ///
    /// Run the `javac` compiler to compile the specified source files for
    /// the configured Android Platform.
    pub fn run(&self) -> Result<(), Error> {
        // Set up basic `javac` command.
        let mut cmd = self.jdk.javac();

        // Append the class-path.
        cmd.arg("--class-path");
        cmd.arg(
            android::sdk::class_path(self.class_paths.clone())
                .map_err(|v| Error::UnsupportedPath(v))?,
        );

        // Select a suitable output directory.
        cmd.arg("-d");
        cmd.arg(self.output_dir);

        // Do not create class-files for implicitly loaded sources. We always
        // specify all sources explicitly, so we want to see linker errors if
        // any external sources are pulled in.
        cmd.arg("-implicit:none");

        // Append all source paths. We ensure they start with a path indicator,
        // since `javac` does not support `--` separators.
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
