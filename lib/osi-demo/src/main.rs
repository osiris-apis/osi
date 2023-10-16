//! Osiris Demo Application
//!
//! Run the Osiris Demo Application. This is implemented in the accompanying
//! library, so we simply defer to its exported entry-point.

use osi_demo;

fn main() -> std::process::ExitCode {
    osi_demo::osi_demo()
}
