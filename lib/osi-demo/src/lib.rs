//! # Osiris Demo Application Library
//!
//! This library provides the implementation of the Osiris Demo Application. It
//! renders a simple UI to test and debug Osiris APIs. It uses the platform
//! native APIs to render the UI and interact with the system.

mod platform;

#[cfg(target_os = "linux")]
fn run() -> std::process::ExitCode {
    let app = platform::linux_fdo::App::new();

    app.run()
}

#[cfg(target_os = "windows")]
fn run() -> std::process::ExitCode {
    let app = platform::windows::App::new();

    app.run()
}

pub fn osi_demo() -> std::process::ExitCode {
    run()
}
