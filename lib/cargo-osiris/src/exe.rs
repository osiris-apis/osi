//! # Executable Entry Points
//!
//! This module exposes the entry-points of the executables shipped with
//! the build system.
//!
//! This module implements command-line interfaces for the wide range of
//! operations exposed by the library. This module does not implement any of
//! the operations, but merely uses the APIs from the library.

use clap;
use crate::{cargo, config, op, toml};

/// ## Cargo Osiris
///
/// This is the entry-point of `cargo-osiris`, the main command-line tool to
/// interact with the Osiris Build System. It can be invoked as a standalone
/// tool or via `cargo osiris ...`.
pub fn cargo_osiris() -> std::process::ExitCode {
    struct Cli {
        cmd: clap::Command,
    }

    impl Cli {
        fn new() -> Self {
            let mut cmd;

            cmd = clap::Command::new("cargo-osiris")
                .propagate_version(true)
                .subcommand_required(true)
                .about("Osiris Build System")
                .long_about("Build system for Rust Applications")
                .version(clap::crate_version!());

            cmd = cmd.arg(
                clap::Arg::new("config")
                    .long("config")
                    .value_name("PATH")
                    .help("Path to the Osiris configuration relative to the working directory")
                    .default_value("./osiris.toml")
                    .value_parser(clap::builder::ValueParser::os_string())
            );

            cmd = cmd.subcommand(
                clap::Command::new("build")
                    .about("Build artifacts for the specified platform")
                    .arg(
                        clap::Arg::new("platform")
                            .long("platform")
                            .value_name("NAME")
                            .help("ID of the target platform to operate on")
                            .required(true)
                            .value_parser(clap::builder::ValueParser::string())
                    )
            );

            cmd = cmd.subcommand(
                clap::Command::new("emerge")
                    .about("Create a persisting platform integration")
                    .arg(
                        clap::Arg::new("platform")
                            .long("platform")
                            .value_name("NAME")
                            .help("ID of the target platform to operate on")
                            .required(true)
                            .value_parser(clap::builder::ValueParser::string())
                    )
                    .arg(
                        clap::Arg::new("update")
                            .long("update")
                            .value_name("BOOL")
                            .help("Whether to allow updating an existing platform integration")
                            .default_value("false")
                            .value_parser(clap::builder::ValueParser::bool())
                    )
            );

            Self {
                cmd: cmd,
            }
        }

        // Handle the `--config <...>` argument.
        fn config(
            &self,
            m: &clap::ArgMatches,
        ) -> Result<(toml::Raw, config::Config), u8> {
            // Unwrap the config-path from the argument.
            let config_arg = m.get_raw("config");
            let mut config_iter = config_arg.expect("Cannot acquire config path");
            assert_eq!(config_iter.len(), 1);
            let config_path = config_iter.next().unwrap();

            // Parse the raw representation from the path.
            let raw = match toml::Raw::parse_path(&config_path) {
                Ok(v) => Ok(v),
                Err(toml::Error::File(v)) => {
                    eprintln!("Cannot parse configuration: Failed reading {:?} ({})", config_path, v);
                    Err(1)
                },
                Err(toml::Error::Toml(v, None)) => {
                    eprintln!("Cannot parse configuration: Invalid TOML syntax ({})", v);
                    Err(1)
                },
                Err(toml::Error::Toml(v, Some(span))) => {
                    eprintln!("Cannot parse configuration: Invalid TOML syntax at offset {}:{} ({})", span.start, span.end, v);
                    Err(1)
                },
                Err(toml::Error::Data(v, None)) => {
                    eprintln!("Cannot parse configuration: Invalid TOML content ({})", v);
                    Err(1)
                },
                Err(toml::Error::Data(v, Some(span))) => {
                    eprintln!("Cannot parse configuration: Invalid TOML content at offset {}:{} ({})", span.start, span.end, v);
                    Err(1)
                },
                Err(toml::Error::Version(v)) => {
                    eprintln!("Cannot parse configuration: Unsupported version '{}'", v);
                    Err(1)
                },
            }?;

            // Validate configuration and convert to internal representation.
            let config = match config::Config::from_toml(&raw, &config_path) {
                Ok(v) => Ok(v),
                Err(config::Error::MissingKey(v)) => {
                    eprintln!("Invalid configuration: Missing configuration for '{}'", v);
                    Err(1)
                },
                Err(config::Error::DuplicatePlatform(v)) => {
                    eprintln!("Invalid configuration: Duplicate platform with ID '{}'", v);
                    Err(1)
                },
            }?;

            Ok((raw, config))
        }

        // Query Cargo for package metadata.
        fn metadata(
            &self,
            config: &config::Config,
        ) -> Result<cargo::Metadata, u8> {
            // Build query parameters.
            let query = cargo::MetadataQuery {
                workspace: config.path_application.clone(),
                main_package: config.package.clone(),
                target: None,
            };

            // Run `cargo metadata` and parse the output.
            match query.run() {
                Err(cargo::Error::Exec(v)) => {
                    eprintln!("Cannot query cargo metadata: Execution of cargo could not commence ({})", v);
                    Err(1)
                },
                Err(cargo::Error::Cargo(v)) => {
                    eprintln!("Cannot query cargo metadata: Cargo failed executing ({})", v);
                    Err(1)
                },
                Err(cargo::Error::Unicode(error)) => {
                    eprintln!("Cannot query cargo metadata: Cargo returned invalid unicode data ({})", error);
                    Err(1)
                },
                Err(cargo::Error::Json) => {
                    eprintln!("Cannot query cargo metadata: Cargo returned invalid JSON data");
                    Err(1)
                },
                Err(cargo::Error::UnknownPackage(v)) => {
                    eprintln!("Cannot query cargo metadata: Requested package name '{}' is unknown", v);
                    Err(1)
                },
                Err(cargo::Error::AmbiguousPackage(v)) => {
                    eprintln!("Cannot query cargo metadata: Requested package name '{}' is ambiguous", v);
                    Err(1)
                },
                Err(cargo::Error::Data) => {
                    eprintln!("Cannot query cargo metadata: Cargo metadata lacks required fields");
                    Err(1)
                },
                Ok(v) => {
                    Ok(v)
                },
            }
        }

        // Handle the `--platform <...>` argument.
        fn platform<'config>(
            &self,
            m: &clap::ArgMatches,
            config: &'config config::Config,
        ) -> Result<&'config config::ConfigPlatform, u8> {
            let id: &String = m.get_one("platform").expect("Cannot acquire platform ID");

            match config.platforms.get(id) {
                None => {
                    eprintln!("No platform with ID {}", id);
                    Err(1)
                },
                Some(v) => Ok(v),
            }
        }

        fn op_build(
            &self,
            m: &clap::ArgMatches,
            m_op: &clap::ArgMatches,
        ) -> Result<(), u8> {
            let (_, config) = self.config(m)?;
            let metadata = self.metadata(&config)?;
            let platform = self.platform(m_op, &config)?;

            match op::build(
                &config,
                &metadata,
                platform,
            ) {
                Err(op::BuildError::DirectoryCreation(dir)) => {
                    eprintln!("Cannot build platform integration: Failed to create directory {:?}", dir);
                    Err(1)
                },
                Err(op::BuildError::Exec(cmd, error)) => {
                    eprintln!("Cannot build platform integration: Failed to invoke '{}' ({})", cmd, error);
                    Err(1)
                },
                Err(op::BuildError::Build) => {
                    eprintln!("Cannot build platform integration: Platform build failed");
                    Err(1)
                },
                Ok(_) => {
                    Ok(())
                },
            }
        }

        fn op_emerge(
            &self,
            m: &clap::ArgMatches,
            m_op: &clap::ArgMatches,
        ) -> Result<(), u8> {
            let (_, config) = self.config(m)?;
            let platform = self.platform(m_op, &config)?;
            let update = *m_op.get_one("update").expect("Update-flag lacks a value");

            match op::emerge(
                &config,
                platform,
                None,
                update,
            ) {
                Err(op::EmergeError::Already) => {
                    eprintln!("Cannot emerge platform integration: Platform code already present");
                    Err(1)
                },
                Err(op::EmergeError::PlatformDirectory(dir)) => {
                    eprintln!("Cannot emerge platform integration: Failed to access platform directory {:?}", dir);
                    Err(1)
                },
                Err(op::EmergeError::DirectoryCreation(dir)) => {
                    eprintln!("Cannot emerge platform integration: Failed to create directory {:?}", dir);
                    Err(1)
                },
                Err(op::EmergeError::FileUpdate(file, error)) => {
                    eprintln!("Cannot emerge platform integration: Failed to update {:?} ({})", file, error);
                    Err(1)
                },
                Err(op::EmergeError::FileRemoval(file, error)) => {
                    eprintln!("Cannot emerge platform integration: Failed to remove {:?} ({})", file, error);
                    Err(1)
                },
                Ok(_) => {
                    Ok(())
                },
            }
        }

        fn run(mut self) -> Result<(), u8> {
            let (m, r);

            r = self.cmd.try_get_matches_from_mut(
                std::env::args_os(),
            );

            match r {
                Ok(v) => m = v,
                Err(e) => {
                    return match e.kind() {
                        clap::error::ErrorKind::DisplayHelp |
                        clap::error::ErrorKind::DisplayVersion => {
                            e.print().expect("Cannot write to STDERR");
                            Ok(())
                        },
                        clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand |
                        _ => {
                            e.print().expect("Cannot write to STDERR");
                            Err(2)
                        }
                    }
                }
            }

            match m.subcommand() {
                Some(("build", m_op)) => self.op_build(&m, &m_op),
                Some(("emerge", m_op)) => self.op_emerge(&m, &m_op),
                _ => std::unreachable!(),
            }
        }
    }

    match Cli::new().run() {
        Ok(()) => 0.into(),
        Err(v) => v.into(),
    }
}
