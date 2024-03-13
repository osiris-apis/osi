//! # Executable Entry Points
//!
//! This module exposes the entry-points of the executables shipped with
//! the build system.
//!
//! This module implements command-line interfaces for the wide range of
//! operations exposed by the library. This module does not implement any of
//! the operations, but merely uses the APIs from the library.

use crate::{cargo, config, lib, op, platform};

/// Application entry-point of cargo-osiris.
///
/// This is the entry-point to the build-system command-line tool of Osiris. It
/// is used to interact with the Osiris Build System. It can be invoked as a
/// standalone tool or via `cargo osiris ...`.
pub fn cargo_osiris() -> std::process::ExitCode {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Cmd {
        Root,
        Archive,
        Build,
    }

    struct Cli {
    }

    impl Cli {
        fn new() -> Self {
            Self {
            }
        }

        // Build configuraton from Cargo metadata.
        fn config(
            &self,
            cargo_arguments: &cargo::Arguments,
        ) -> Result<(cargo::Metadata, config::Config), u8> {
            let manifest_path = cargo_arguments.manifest_path();

            // Build query parameters.
            let query = cargo::MetadataQuery {
                cargo_arguments: cargo_arguments,
                target: None,
            };

            // Run `cargo metadata` and parse the output.
            let metadata = match query.run() {
                Ok(v) => {
                    Ok(v)
                },
                Err(e) => {
                    eprintln!("Cannot query cargo metadata: {}", e);
                    Err(1)
                },
            }?;

            // Build internal configuration based on the metadata.
            let config = match config::Config::from_cargo(&metadata, &manifest_path) {
                Ok(v) => Ok(v),
                Err(e) => {
                    eprintln!("Cannot build configuration: {}", e);
                    Err(1)
                },
            }?;

            Ok((metadata, config))
        }

        // Handle the `--archive <...>` argument.
        fn archive<'config>(
            &self,
            config: &'config config::Config,
            v_archive: &Option<String>,
        ) -> Result<&'config config::ConfigArchive, u8> {
            let id = match v_archive {
                None => {
                    eprintln!("No archive configuration specified");
                    Err(1)
                },
                Some(ref v) => Ok(v),
            }?;

            match config.archive(id) {
                None => {
                    eprintln!("No archive configuration with ID {}", id);
                    Err(1)
                },
                Some(v) => Ok(v),
            }
        }

        // Handle the `--platform <...>` argument.
        fn platform<'config>(
            &self,
            config: &'config config::Config,
            v_platform: &Option<String>,
        ) -> Result<&'config config::ConfigPlatform, u8> {
            let id = match v_platform {
                None => {
                    eprintln!("No platform integration specified");
                    Err(1)
                },
                Some(ref v) => Ok(v),
            }?;

            match config.platform(id) {
                None => {
                    eprintln!("No platform integration with ID {}", id);
                    Err(1)
                },
                Some(v) => Ok(v),
            }
        }

        fn op_archive(
            &self,
            v_archive: &Option<String>,
            v_platform: &Option<String>,
            verbose: bool,
            cargo_arguments: &cargo::Arguments,
        ) -> Result<(), u8> {
            let (metadata, config) = self.config(cargo_arguments)?;
            let archive = self.archive(&config, v_archive)?;
            let platform = self.platform(&config, v_platform)?;
            let op = op::Archive {
                archive: &archive,
                cargo_arguments: cargo_arguments,
                cargo_metadata: &metadata,
                config: &config,
                platform: &platform,
                verbose: verbose,
            };

            match op.run() {
                Ok(()) => {
                    Ok(())
                },
                Err(e) => {
                    eprintln!("Cannot build archive: {}", e);
                    Err(1)
                },
            }
        }

        fn op_build(
            &self,
            v_platform: &Option<String>,
            verbose: bool,
            cargo_arguments: &cargo::Arguments,
        ) -> Result<(), u8> {
            let (metadata, config) = self.config(cargo_arguments)?;
            let platform = self.platform(&config, v_platform)?;
            let build = op::Build {
                cargo_arguments: cargo_arguments,
                cargo_metadata: &metadata,
                config: &config,
                platform: &platform,
                verbose: verbose,
            };

            match build.build() {
                Err(op::BuildError::Uncaught(v)) => {
                    eprintln!("Cannot build platform integration: Uncaught failure: {}", v);
                    Err(1)
                },
                Err(op::BuildError::FileSystem(v)) => {
                    eprintln!("Cannot build platform integration: {}", v);
                    Err(1)
                },
                Err(op::BuildError::Process(v)) => {
                    eprintln!("Cannot build platform integration: {}", v);
                    Err(1)
                },
                Err(op::BuildError::Exec(tool, v)) => {
                    eprintln!("Cannot build platform integration: Execution of {} could not commence: {}", tool, v);
                    Err(1)
                },
                Err(op::BuildError::Exit(tool, v)) => {
                    eprintln!("Cannot build platform integration: {} failed executing: {}", tool, v);
                    Err(1)
                },
                Err(op::BuildError::Cargo(e)) => {
                    eprintln!("Cannot build Android platform integration: {}", e);
                    Err(1)
                },
                Err(op::BuildError::AndroidPlatform(sub)) => match sub {
                    platform::android::BuildError::UnsupportedPath(path) => {
                        eprintln!("Cannot build Android platform integration: Path contains characters not supported by Android Builds (e.g., colons, semicolons): {}", path.display());
                        Err(1)
                    },
                    platform::android::BuildError::UnsupportedHost => {
                        eprintln!("Cannot build Android platform integration: Host platform not supported by the Android SDK");
                        Err(1)
                    },
                    platform::android::BuildError::UnsupportedAbi(v) => {
                        eprintln!("Cannot build Android platform integration: Selected target ABI is not supported: {}", v);
                        Err(1)
                    },
                    platform::android::BuildError::NoAndroidHome => {
                        eprintln!("Cannot build Android platform integration: No Android SDK available, `ANDROID_HOME` is not set");
                        Err(1)
                    },
                    platform::android::BuildError::NoSdk(path) => {
                        eprintln!("Cannot build Android platform integration: No Android SDK at {}", path.display());
                        Err(1)
                    },
                    platform::android::BuildError::InvalidSdk(path) => {
                        eprintln!("Cannot build Android platform integration: Invalid Android SDK at {}", path.display());
                        Err(1)
                    },
                    platform::android::BuildError::NoJdk(path) => {
                        eprintln!("Cannot build Android platform integration: No Android Java SDK at {}", path.display());
                        Err(1)
                    },
                    platform::android::BuildError::InvalidJdk(path) => {
                        eprintln!("Cannot build Android platform integration: Invalid Android Java SDK at {}", path.display());
                        Err(1)
                    },
                    platform::android::BuildError::NoKdk(path) => {
                        eprintln!("Cannot build Android platform integration: No Android Kotlin SDK at {}", path.display());
                        Err(1)
                    },
                    platform::android::BuildError::InvalidKdk(path) => {
                        eprintln!("Cannot build Android platform integration: Invalid Android Kotlin SDK at {}", path.display());
                        Err(1)
                    },
                    platform::android::BuildError::NoNdk => {
                        eprintln!("Cannot build Android platform integration: Android SDK lacks NDK component");
                        Err(1)
                    },
                    platform::android::BuildError::InvalidNdk(v) => {
                        eprintln!("Cannot build Android platform integration: No valid NDK of the selected version available in the Android SDK: {:?}", v);
                        Err(1)
                    },
                    platform::android::BuildError::NoBuildTools => {
                        eprintln!("Cannot build Android platform integration: Android SDK lacks Build Tools");
                        Err(1)
                    },
                    platform::android::BuildError::InvalidBuildTools(v) => {
                        eprintln!("Cannot build Android platform integration: No valid Build Tools of the selected version available in the Android SDK: {:?}", v);
                        Err(1)
                    },
                    platform::android::BuildError::NoPlatform(v) => {
                        eprintln!("Cannot build Android platform integration: Android SDK lacks Platform for API-level {}", v);
                        Err(1)
                    },
                    platform::android::BuildError::InvalidPlatform(v) => {
                        eprintln!("Cannot build Android platform integration: No valid Platform for API-level {} available in the Android SDK", v);
                        Err(1)
                    },
                    platform::android::BuildError::FlatresExec(v) => {
                        eprintln!("Cannot build Android platform integration: Execution of Android flatres compiler could not commence: {}", v);
                        Err(1)
                    },
                    platform::android::BuildError::FlatresExit(v) => {
                        eprintln!("Cannot build Android platform integration: Android flatres compiler failed executing: {}", v);
                        Err(1)
                    },
                    platform::android::BuildError::JavacExec(v) => {
                        eprintln!("Cannot build Android platform integration: Execution of Java compiler could not commence: {}", v);
                        Err(1)
                    },
                    platform::android::BuildError::JavacExit(v) => {
                        eprintln!("Cannot build Android platform integration: Java compiler failed executing: {}", v);
                        Err(1)
                    },
                    platform::android::BuildError::KotlincExec(v) => {
                        eprintln!("Cannot build Android platform integration: Execution of Kotlin compiler could not commence: {}", v);
                        Err(1)
                    },
                    platform::android::BuildError::KotlincExit(v) => {
                        eprintln!("Cannot build Android platform integration: Kotlin compiler failed executing: {}", v);
                        Err(1)
                    },
                    platform::android::BuildError::DexExec(v) => {
                        eprintln!("Cannot build Android platform integration: Execution of DEX compiler could not commence: {}", v);
                        Err(1)
                    },
                    platform::android::BuildError::DexExit(v) => {
                        eprintln!("Cannot build Android platform integration: DEX compiler failed executing: {}", v);
                        Err(1)
                    },
                    platform::android::BuildError::AaptExec(v) => {
                        eprintln!("Cannot build Android platform integration: Execution of Android APK linker could not commence: {}", v);
                        Err(1)
                    },
                    platform::android::BuildError::AaptExit(v) => {
                        eprintln!("Cannot build Android platform integration: Android APK linker failed executing: {}", v);
                        Err(1)
                    },
                },
                Err(op::BuildError::MacosPlatform(sub)) => {
                    eprintln!("Cannot build macOS platform integration: {}", sub);
                    Err(1)
                },
                Ok(_) => {
                    Ok(())
                },
            }
        }

        fn run(&self) -> Result<(), u8> {
            use crate::lib::args::{Flag, Value};

            let cwd = std::env::current_dir().expect("Current working directory must be available");

            let args = std::env::args_os().skip(1).collect::<Vec<std::ffi::OsString>>();

            let v_archive: core::cell::RefCell<Option<String>> = Default::default();
            let v_help = lib::args::Help::new();
            let v_platform: core::cell::RefCell<Option<String>> = Default::default();
            let v_verbose: core::cell::RefCell<Option<bool>> = Default::default();

            let v_default_features: core::cell::RefCell<Option<bool>> = Default::default();
            let v_features: core::cell::RefCell<Vec<&str>> = Default::default();
            let v_frozen: core::cell::RefCell<Option<bool>> = Default::default();
            let v_manifest_path: core::cell::RefCell<Option<std::ffi::OsString>> = Default::default();
            let v_package: core::cell::RefCell<Option<String>> = Default::default();
            let v_profile: core::cell::RefCell<Option<String>> = Default::default();
            let v_target_dir: core::cell::RefCell<Option<std::ffi::OsString>> = Default::default();

            let flags_build = lib::args::FlagList::with([
                Flag::with_name("help", Value::Set(&v_help), Some("Show usage information")),
                Flag::with_name("platform", Value::Parse(&v_platform), Some("ID of the target platform")),
                Flag::with_name("verbose", Value::Parse(&v_verbose), Some("Be more verbose")),

                Flag::with_name("default-features", Value::Toggle(&v_default_features), Some("Enable/Disable default package features")),
                Flag::with_name("features", Value::Parse(&v_features), Some("Enable specified package features")),
                Flag::with_name("frozen", Value::Parse(&v_frozen), Some("Use `Cargo.lock` without checking for updates")),
                Flag::with_name("manifest-path", Value::Parse(&v_manifest_path), Some("Path to `Cargo.toml`")),
                Flag::with_name("package", Value::Parse(&v_package), Some("Workspace package to build")),
                Flag::with_name("profile", Value::Parse(&v_profile), Some("Name of the build profile")),
                Flag::with_name("target-dir", Value::Parse(&v_target_dir), Some("Path to the target directory")),
            ]);
            let flags_archive = lib::args::FlagList::with([
                Flag::with_name("archive", Value::Parse(&v_archive), Some("ID of the target archive")),
                Flag::with_name("help", Value::Set(&v_help), Some("Show usage information")),
                Flag::with_name("platform", Value::Parse(&v_platform), Some("ID of the target platform")),
                Flag::with_name("verbose", Value::Parse(&v_verbose), Some("Be more verbose")),

                Flag::with_name("default-features", Value::Toggle(&v_default_features), Some("Enable/Disable default package features")),
                Flag::with_name("features", Value::Parse(&v_features), Some("Enable specified package features")),
                Flag::with_name("frozen", Value::Parse(&v_frozen), Some("Use `Cargo.lock` without checking for updates")),
                Flag::with_name("manifest-path", Value::Parse(&v_manifest_path), Some("Path to `Cargo.toml`")),
                Flag::with_name("package", Value::Parse(&v_package), Some("Workspace package to build")),
                Flag::with_name("profile", Value::Parse(&v_profile), Some("Name of the build profile")),
                Flag::with_name("target-dir", Value::Parse(&v_target_dir), Some("Path to the target directory")),
            ]);
            let flags_root = lib::args::FlagList::with([
                Flag::with_name("help", Value::Set(&v_help), Some("Show usage information")),
            ]);

            let cmds_root = lib::args::CommandList::with([
                lib::args::Command::with_name(
                    Cmd::Archive, "archive", Default::default(), &flags_archive, None,
                    Some("Build archives for the specified platform"),
                ),
                lib::args::Command::with_name(
                    Cmd::Build, "build", Default::default(), &flags_build, None,
                    Some("Build artifacts for the specified platform"),
                ),
            ]);

            let root = lib::args::Command::with_name(
                Cmd::Root, "cargo-osiris", &cmds_root, &flags_root, None,
                Some("Osiris Apis Build System"),
            );

            let r_cmd = lib::args::Parser::new().parse(
                args.iter().map(|v| lib::compat::OsStr::from_osstr(v.as_os_str())),
                &root,
            );

            let mut fmt_stderr = lib::compat::Write(std::io::stderr().lock());
            let mut fmt_stdout = lib::compat::Write(std::io::stderr().lock());

            // Handle all errors of the command-line parser. Note that we get
            // a batch of errors, which we all propagate to the user.
            let cmd = match r_cmd {
                Ok(v) => v,
                Err(errors) => {
                    eprintln!("Cannot parse command-line arguments:");
                    for e in errors.iter() {
                        eprintln!("- {}", e);
                    }
                    return Err(2);
                },
            };

            // If `--help` was requested, show usage information on `stdout`
            // and exit with success.
            if v_help
                .help(&root, &mut fmt_stdout)
                .expect("STDERR must be writable")
            {
                return Ok(());
            }

            match cmd {
                Cmd::Root => {
                    // If a non-selectable command was chosen, print usage
                    // information on `stderr` and return failure.
                    lib::args::Help::help_for(&root, &mut fmt_stderr, &cmd)
                        .expect("STDERR must be writable");
                    Err(2)
                },
                Cmd::Archive => self.op_archive(
                    &*v_archive.borrow(),
                    &*v_platform.borrow(),
                    v_verbose.borrow().unwrap_or(false),
                    &cargo::Arguments {
                        default_features: *v_default_features.borrow(),
                        features: v_features.borrow().iter().map(|v| (*v).into()).collect(),
                        frozen: *v_frozen.borrow(),
                        manifest_path: v_manifest_path.borrow().as_ref()
                            .map(|v| cwd.join(v)),
                        package: v_package.borrow().clone(),
                        profile: v_profile.borrow().clone(),
                        target_dir: v_target_dir.borrow().as_ref()
                            .map(|v| cwd.join(v)),
                    },
                ),
                Cmd::Build => self.op_build(
                    &*v_platform.borrow(),
                    v_verbose.borrow().unwrap_or(false),
                    &cargo::Arguments {
                        default_features: *v_default_features.borrow(),
                        features: v_features.borrow().iter().map(|v| (*v).into()).collect(),
                        frozen: *v_frozen.borrow(),
                        manifest_path: v_manifest_path.borrow().as_ref()
                            .map(|v| cwd.join(v)),
                        package: v_package.borrow().clone(),
                        profile: v_profile.borrow().clone(),
                        target_dir: v_target_dir.borrow().as_ref()
                            .map(|v| cwd.join(v)),
                    },
                ),
            }
        }
    }

    match Cli::new().run() {
        Ok(()) => 0.into(),
        Err(v) => v.into(),
    }
}
