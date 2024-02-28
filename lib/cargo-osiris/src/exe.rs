//! # Executable Entry Points
//!
//! This module exposes the entry-points of the executables shipped with
//! the build system.
//!
//! This module implements command-line interfaces for the wide range of
//! operations exposed by the library. This module does not implement any of
//! the operations, but merely uses the APIs from the library.

use crate::{cargo, config, lib, op, platform};

/// ## Cargo Osiris
///
/// This is the entry-point of `cargo-osiris`, the main command-line tool to
/// interact with the Osiris Build System. It can be invoked as a standalone
/// tool or via `cargo osiris ...`.
pub fn cargo_osiris() -> std::process::ExitCode {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Cmd {
        Root,
        Build,
    }

    struct Cli {
    }

    impl Cli {
        fn new() -> Self {
            Self {
            }
        }

        // Query Cargo for package metadata.
        fn metadata(
            &self,
            v_manifest: &Option<String>,
        ) -> Result<(cargo::Metadata, config::Config), u8> {
            let manifest_path: std::path::PathBuf = v_manifest.as_deref().unwrap_or(".").into();

            // Build query parameters.
            let query = cargo::MetadataQuery {
                workspace: manifest_path.clone(),
                package: None,
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
            let config = match config::Config::from_cargo(&metadata.osiris, &manifest_path) {
                Ok(v) => Ok(v),
                Err(e) => {
                    eprintln!("Cannot build configuration: {}", e);
                    Err(1)
                },
            }?;

            Ok((metadata, config))
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

            match config.platforms.get(id) {
                None => {
                    eprintln!("No platform integration with ID {}", id);
                    Err(1)
                },
                Some(v) => Ok(v),
            }
        }

        fn op_build(
            &self,
            v_manifest: &Option<String>,
            v_platform: &Option<String>,
        ) -> Result<(), u8> {
            let (metadata, config) = self.metadata(v_manifest)?;
            let platform = self.platform(&config, v_platform)?;

            match op::build(
                &config,
                &metadata,
                platform,
            ) {
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
                Ok(_) => {
                    Ok(())
                },
            }
        }

        fn run(&self) -> Result<(), u8> {
            use crate::lib::args::{Flag, Value};

            let args = std::env::args_os().skip(1).collect::<Vec<std::ffi::OsString>>();

            let v_help = lib::args::Help::new();
            let v_manifest: core::cell::RefCell<Option<String>> = Default::default();
            let v_platform: core::cell::RefCell<Option<String>> = Default::default();

            let flags_build = lib::args::FlagList::with([
                Flag::with_name("help", Value::Set(&v_help), Some("Show usage information")),
                Flag::with_name("platform", Value::Parse(&v_platform), Some("ID of the target platform")),
            ]);
            let flags_root = lib::args::FlagList::with([
                Flag::with_name("help", Value::Set(&v_help), Some("Show usage information")),
                Flag::with_name("manifest", Value::Parse(&v_manifest), Some("Path to the Cargo manifest")),
            ]);

            let cmds_root = lib::args::CommandList::with([
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
                Cmd::Build => self.op_build(
                    &*v_manifest.borrow(),
                    &*v_platform.borrow(),
                ),
            }
        }
    }

    match Cli::new().run() {
        Ok(()) => 0.into(),
        Err(v) => v.into(),
    }
}
