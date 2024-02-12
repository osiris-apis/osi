//! Osiris Apis Analyzer Application
//!
//! Run the Osiris Apis Analyzer Application. This is implemented in the
//! accompanying library, so we simply defer to its exported entry-point.

use osiris_analyzer;

fn main() -> std::process::ExitCode {
    osiris_analyzer::osiris_analyzer()
}
