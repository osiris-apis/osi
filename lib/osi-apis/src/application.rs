//! # Application Context
//!
//! The application context represents the local application and provides
//! access to system APIs that manage or track the application. It is used
//! by several modules to interact with the system representation of the
//! local application.

/// ## Application Setup
///
/// The setup structure contains all the parameters required to initialize
/// the application.
#[derive(Default)]
pub struct Setup<'ctx> {
    /// Parameters required by the native application setup.
    pub native: crate::native::application::Setup<'ctx>,
    /// Output directory of the package. Usually initialized via
    /// `option_env!("OUT_DIR")`. Set to `None` if no generated package
    /// data is used.
    pub out_dir: Option<&'ctx std::path::Path>,
    /// Name of the package. Usually initialized via
    /// `env!("CARGO_PKG_NAME")`. Used to validate the environment when
    /// running from source checkouts. Set to `None` to disable this
    /// validation.
    pub package: Option<&'ctx str>,
}

/// ## Application Context
///
/// The context of the local application, providing access to system APIs
/// regarding the state and lifetime of the application.
pub struct Context {
    pub(crate) native: crate::native::application::Context,
    pub(crate) out_dir: Option<std::path::PathBuf>,
    pub(crate) package: Option<String>,
}

impl<'ctx> From<crate::native::application::Setup<'ctx>> for Setup<'ctx> {
    fn from(v: crate::native::application::Setup<'ctx>) -> Self {
        Self {
            native: v,
            ..Default::default()
        }
    }
}

impl<'ctx> Setup<'ctx> {
    /// ## Create New Setup Object
    ///
    /// Create a new setup object with all the default values set.
    pub fn new() -> Self {
        Default::default()
    }

    /// ## Initialize the Application
    ///
    /// Perform all application initialization and yield the application
    /// context ready to be used.
    pub fn initialize(
        &self,
    ) -> Result<Context, Box<dyn std::error::Error>> {
        let v_native = self.native.initialize()?;

        Ok(Context {
            native: v_native,
            out_dir: self.out_dir.map(|v| v.into()),
            package: self.package.map(|v| v.into()),
        })
    }
}

impl Context {
    /// ## Yield Native Context
    ///
    /// Return a reference to the underlying native context.
    pub fn native(&self) -> &crate::native::application::Context {
        &self.native
    }

    /// ## Locate Application Data Directory
    ///
    /// Yield a path to the root of the application data directory. If no such
    /// directory exists, `None` is returned.
    ///
    /// If the application is run from a development checkout, this will yield
    /// a path to either the source root directory (`is_static` is true), or
    /// the build root directory (`is_static` is false) of the Rust package
    /// where the entry-point of the application resides.
    ///
    /// If the application is run from an install (packaged or unpackaged),
    /// this will yield a path to the root directory of the install.
    ///
    /// The `subdir_source` and `subdir_install` paths are joined with the
    /// yielded path. `subdir_source` is used for paths pointing to the source
    /// directory of the package, `subdir_install` is used for paths pointing
    /// to the build or install directory.
    pub fn locate_data(
        &self,
        is_static: bool,
        subdir_source: Option<&std::path::Path>,
        subdir_install: Option<&std::path::Path>,
    ) -> Option<std::path::PathBuf> {
        let dot = std::path::Path::new(".");
        let subdir_source = subdir_source.unwrap_or_else(|| dot);
        let subdir_install = subdir_install.unwrap_or_else(|| dot);

        // For development builds we support running directly from within
        // `cargo run`. We detect this by checking for `CARGO_MANIFEST_DIR`,
        // which is set by `cargo run` and points to the source directory.
        // If it is not set, we assume we are run via other means, and thus
        // use the logic for packaged builds.
        //
        // We also verify `CARGO_PKG_NAME` matches the expected package
        // name, to ensure we do not accidentally read the environment when
        // invoked from another package run through `cargo run`. This requires
        // that the caller sets `Setup::package` accordingly, though.
        //
        // We disable this entire logic if a target packaging format is set.
        // This means, when building for a specific application format, we
        // do not support running from within `cargo run`, but only through
        // the standard invocation options of the selected target packaging
        // format. This prevents manipulated invocations with custom
        // environments by a user, and thus causing disrupted application
        // setups. This is not necessarily a security issue, since the
        // caller must always assume the return path is under user control.
        // However, it prevents accidental failure due to inherited
        // environments.
        //
        // Note that `target_format` is not standardized by Cargo so far. It
        // has to be set manually by packaging tools when targetting a
        // specific platform.
        fn local(
            setup: &Context,
            is_static: bool,
            subdir_source: &std::path::Path,
            subdir_install: &std::path::Path,
        ) -> Option<std::path::PathBuf> {
            if cfg!(target_format) {
                return None;
            }
            if let Some(ref package) = setup.package {
                let pkg = <_ as AsRef<std::ffi::OsStr>>::as_ref(package);
                let env = match std::env::var_os("CARGO_PKG_NAME") {
                    Some(v) => v,
                    _ => return None,
                };
                if env.as_os_str() != pkg {
                    return None;
                }
            }
            if is_static {
                match std::env::var_os("CARGO_MANIFEST_DIR") {
                    Some(v) => Some(std::path::Path::new(&v).join(subdir_source)),
                    _ => None,
                }
            } else {
                match setup.out_dir {
                    Some(ref v) => Some(v.join(subdir_install)),
                    _ => None,
                }
            }
        }

        if let Some(v) = local(self, is_static, subdir_source, subdir_install) {
            return Some(v);
        }

        // XXX: We need to evaluate how the different platforms provide access
        //      to the install directory. For now, we use the directory where
        //      the process is invoked, but this definitely fails for
        //      out-of-tree invocations.
        Some(
            std::env::current_dir()
                .unwrap_or_else(|_| dot.into())
                .join(subdir_install)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_location() {
        let app = Setup {
            out_dir: std::option_env!("OUT_DIR")
                .map(|v| std::path::Path::new(v)),
            package: Some(std::env!("CARGO_PKG_NAME")),
            ..Default::default()
        }.initialize().unwrap();

        assert_eq!(
            app.locate_data(true, None, None),
            Some(std::env!("CARGO_MANIFEST_DIR").into()),
        );
    }
}
