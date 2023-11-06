//! # Cargo Osiris
//!
//! This executable is the main entrypoint of the Osiris Build System. It is
//! meant as sub-command of `cargo` and to be invoked as `cargo osiris ...`.
//!
//! This executable simply calls into `cargo_osiris::cargo_osiris()` of the
//! accompanying library.

use cargo_osiris;

fn main() -> std::process::ExitCode {
    cargo_osiris::cargo_osiris()
}
