//! Osiris Apis Analyzer Runner
//!
//! This runner is a helper binary that simply invokes the entry-point of
//! the Osiris Apis Analyzer (implemented in the library). The runner is
//! not used for deployments, but allows running from a checkout directly
//! via `cargo run`.
//!
//! If Cargo supports `cargo run --lib` (with `crate-type = ["bin"]`),
//! this runner will become obsolete. Until then, use it for running directly
//! from the checkout.

use osiris_analyzer;

fn main() -> std::process::ExitCode {
    osiris_analyzer::osiris_analyzer()
}
