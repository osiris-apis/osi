//! # Osiris Apis Analyzer Library
//!
//! This library provides the implementation of the Osiris Apis Analyzer
//! Application. It renders a simple UI to analyze Osiris APIs. It uses the
//! platform native APIs to render the UI and interact with the system.

mod platform;

#[cfg(target_os = "linux")]
fn run() -> std::process::ExitCode {
    let app = platform::linux_fdo::App::new();

    app.run()
}

#[cfg(target_os = "macos")]
fn run() -> std::process::ExitCode {
    let app = platform::macos::App::new();

    app.run()
}

#[cfg(target_os = "windows")]
fn run() -> std::process::ExitCode {
    let app = platform::windows::App::new();

    app.run()
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "windows",
)))]
fn run() -> std::process::ExitCode {
    1.into()
}

pub fn osiris_analyzer() -> std::process::ExitCode {
    run()
}

// Entry-point to the application, in case this library is compiled as
// `crate-type` set to `bin`. This is the case for all platforms that
// need the entry-point as executable. For platforms that need entry-points
// as shared library, this will be a stub that is removed by the linker.
#[allow(dead_code)]
fn main() -> std::process::ExitCode {
    osiris_analyzer()
}
