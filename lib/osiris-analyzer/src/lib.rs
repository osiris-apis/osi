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
