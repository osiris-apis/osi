//! # Android Platform Flat Resources
//!
//! The Android Platform can compile a wide range of resource files into a
//! format with the `.flat` extension. This is used to prepare resources for
//! fast lookups before assembling an APK. This module provides helpers to
//! deal with `aapt2`, the compiler for flat resource files.

use crate::platform::android;

/// ## Compilation Error
///
/// This is the error-enum of all possible errors raised by this
/// compilation abstraction.
#[derive(Debug)]
pub enum Error {
    /// Invalid resource path (must include resource directory and resource
    /// file).
    InvalidPath(std::path::PathBuf),
    /// Program execution failed with the given error.
    Exec(std::io::Error),
    /// Program exited with a failure condition.
    Exit(std::process::ExitStatus),
}

/// ## Flat Resource Compiler Query
///
/// This represents the parameters to a flat resource compilation. It is to
/// be filled in by the caller.
pub struct Query {
    /// Android SDK build tools to use for the compilation.
    pub build_tools: android::sdk::BuildTools,
    /// Output directory where to store the flat resource files.
    pub output_dir: std::path::PathBuf,
    /// Resource file to compile.
    pub resource_file: std::path::PathBuf,
}

impl Query {
    /// ## Compute Output File Name
    ///
    /// The `aapt2` tool uses fixed names for output files (mostly for
    /// compatibility reasons to `aapt`). In most cases, it simply appends
    /// `.flat` to the file name. However, for some file types it does
    /// some more elaborate logic. We duplicate this so we can reliably
    /// tell which file was produced by `aapt2`.
    ///
    /// Note that this strips leading path information and returns a file
    /// name only. The caller likely has to append this to the output directory.
    ///
    /// This mirrors the behavior of `ExtractResourcePathData()`, as well
    /// as `BuildIntermediateContainerFilename()` in
    /// `tools/aapt2/cmd/Compile.cpp` of `platforms/frameworks/base`.
    pub fn output_file_name(
        path: &std::path::Path,
    ) -> Option<std::ffi::OsString> {
        // XXX: `std::path::Path` normalizes trailing slashes, which is not
        //      really correct for files. Yet, it is unlikely to lead to
        //      issues, so we ignore it. Same is true for trailing dot
        //      components.

        // Extract file stem and extension.
        let (stem, mut ext) = match (
            path.file_stem(),
            path.extension(),
        ) {
            // Error out if the path has no valid file name.
            (None, _) => {
                return None;
            },

            // If no extension is used, skip parsing it.
            (Some(stem), None) => (stem, None),

            // For some extensions `aapt2` may use a 2nd-layer extension. Try
            // to parse those.
            (Some(stem), Some(ext))
            if ext == "png" => {
                let stem_path: &std::path::Path = stem.as_ref();
                match (
                    stem_path.file_stem(),
                    stem_path.extension(),
                ) {
                    // `.9.png` is handled as 2-layered extension.
                    (Some(sub_stem), Some(sub_ext))
                    if ext == "png" && sub_ext == "9" => {
                        (sub_stem, Some("9.png".as_ref()))
                    },

                    // Everything else is a single-layer extension.
                    _ => (stem, Some(ext)),
                }
            },

            // For all other extensions, only a single layer is used.
            (Some(stem), Some(ext)) => (stem, Some(ext)),
        };

        // Extract the resource directory and config-suffix.
        let (dir, config)  = match
            path.parent()
                .filter(|v| v.as_os_str().len() > 0)
                .map(|v| v.file_name())
                .flatten()
        {
            None => {
                return None;
            },
            Some(dir) => {
                // Split at the first dash, if any.
                let bytes = dir.as_encoded_bytes();
                match bytes.iter().position(|v| *v == b'-') {
                    None => (dir, None),
                    Some(idx) => unsafe {
                        (
                            std::ffi::OsStr::from_encoded_bytes_unchecked(&bytes[0..idx]),
                            Some(std::ffi::OsStr::from_encoded_bytes_unchecked(&bytes[idx+1..])),
                        )
                    },
                }
            },
        };

        // Now perform the transformations as done by `aapt2`. This currently
        // means:
        //
        // - XML value files use `arsc` extensions, rather than `xml`, for
        //   historic reasons.
        if dir == "values" {
            if let Some(v) = ext {
                if v == "xml" {
                    ext = Some(std::ffi::OsStr::new("arsc"));
                }
            }
        }

        // Assemble output file name.
        let mut output = std::ffi::OsString::new();

        output.push(dir);
        if let Some(v) = config {
            output.push("-");
            output.push(v);
        }
        output.push("_");
        output.push(stem);
        if let Some(v) = ext {
            output.push(".");
            output.push(v);
        }
        output.push(".flat");

        Some(output)
    }

    /// ## Run `aapt2` compiler
    ///
    /// Run the `aapt2` flat resource compiler, producing a flat resource for
    /// the given resource input.
    pub fn run(&self) -> Result<std::path::PathBuf, Error> {
        let output_file = self.output_dir.join(
            Self::output_file_name(&self.resource_file).ok_or_else(
                || Error::InvalidPath(self.resource_file.clone()),
            )?,
        );

        // Set up basic `aapt2 compile` command.
        let mut cmd = std::process::Command::new(
            self.build_tools.aapt2()
        );
        cmd.args([
            "compile",
        ]);

        // Specify the output directory.
        cmd.arg("-o");
        cmd.arg(self.output_dir.as_path());

        // Append the input resource file.
        //
        // Ensure the path does not start with a dash or other character
        // that might be interpreted by `aapt2`. The `--` separator is not
        // supported, so we instead ensure the path uses a dot-prefix or
        // is absolute.
        cmd.arg(std::path::Path::new(".").join(&self.resource_file));

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

        Ok(output_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify the behavior of the output file name prediction of the
    // `aapt2` flat resource compiler.
    #[test]
    fn output_file_name_basic() {
        // Verify error handling for invalid paths.
        assert!(Query::output_file_name("".as_ref()).is_none());
        assert!(Query::output_file_name("/".as_ref()).is_none());
        assert!(Query::output_file_name("/.".as_ref()).is_none());
        assert!(Query::output_file_name("foo".as_ref()).is_none());
        assert!(Query::output_file_name("foo/".as_ref()).is_none());
        assert!(Query::output_file_name("foo/.".as_ref()).is_none());
        assert!(Query::output_file_name("foo/..".as_ref()).is_none());
        assert!(Query::output_file_name("/foo".as_ref()).is_none());
        assert!(Query::output_file_name("./foo".as_ref()).is_none());
        assert!(Query::output_file_name("../foo".as_ref()).is_none());

        // Basic file name transformations.
        assert_eq!(
            Query::output_file_name("foo/bar".as_ref()).unwrap(),
            <str as AsRef<std::path::Path>>::as_ref("foo_bar.flat"),
        );
        assert_eq!(
            Query::output_file_name("dir/stem.ext".as_ref()).unwrap(),
            <str as AsRef<std::path::Path>>::as_ref("dir_stem.ext.flat"),
        );

        // Verify that leading paths are stripped.
        assert_eq!(
            Query::output_file_name("foo/bar/dir/stem.ext".as_ref()).unwrap(),
            <str as AsRef<std::path::Path>>::as_ref("dir_stem.ext.flat"),
        );

        // Verify that configuration suffixes are retained.
        assert_eq!(
            Query::output_file_name("dir-config/stem.ext".as_ref()).unwrap(),
            <str as AsRef<std::path::Path>>::as_ref("dir-config_stem.ext.flat"),
        );
        assert_eq!(
            Query::output_file_name("dir-more-config/stem.ext".as_ref()).unwrap(),
            <str as AsRef<std::path::Path>>::as_ref("dir-more-config_stem.ext.flat"),
        );

        // Verify handling of `9.png` is analogous to other extensions.
        assert_eq!(
            Query::output_file_name("dir/stem.png".as_ref()).unwrap(),
            <str as AsRef<std::path::Path>>::as_ref("dir_stem.png.flat"),
        );
        assert_eq!(
            Query::output_file_name("dir/stem.9.png".as_ref()).unwrap(),
            <str as AsRef<std::path::Path>>::as_ref("dir_stem.9.png.flat"),
        );

        // Verify that `values/*.xml` uses `*.arsc` extension.
        assert_eq!(
            Query::output_file_name("dir/stem.xml".as_ref()).unwrap(),
            <str as AsRef<std::path::Path>>::as_ref("dir_stem.xml.flat"),
        );
        assert_eq!(
            Query::output_file_name("values/stem.xml".as_ref()).unwrap(),
            <str as AsRef<std::path::Path>>::as_ref("values_stem.arsc.flat"),
        );
        assert_eq!(
            Query::output_file_name("values-/stem.xml".as_ref()).unwrap(),
            <str as AsRef<std::path::Path>>::as_ref("values-_stem.arsc.flat"),
        );
        assert_eq!(
            Query::output_file_name("values-foobar/stem.xml".as_ref()).unwrap(),
            <str as AsRef<std::path::Path>>::as_ref("values-foobar_stem.arsc.flat"),
        );

        // Verify some more complex combinations.
        assert_eq!(
            Query::output_file_name("path/to/values--foo-bar--/file.stem.xml".as_ref()).unwrap(),
            <str as AsRef<std::path::Path>>::as_ref("values--foo-bar--_file.stem.arsc.flat"),
        );

        // Trailing slashes and dot-components are normalized by `std::path`.
        // This is quite unfortunate when dealing with files rather than
        // directories, but little we can do about it. At least verify that
        // this is the case.
        assert_eq!(
            Query::output_file_name("foo/bar/.".as_ref()).unwrap(),
            <str as AsRef<std::path::Path>>::as_ref("foo_bar.flat"),
        );
    }
}
