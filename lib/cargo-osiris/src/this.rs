//! # Process Context
//!
//! This module provides a custom process context with access to all global
//! entities. Any module that needs access to one of these contexts must
//! thus be passed the process context.

use crate::con;

/// Enumeration of user options to control the display mode.
pub enum DisplayOption {
    /// Automatically decide which mode to use, based on display capabilities
    Auto,
    /// Use plain-text mode without styling
    Plain,
    /// Use ANSI-compatible escape sequences for styling
    Ansi,
    /// Use the Windows Console API for styling
    Wincon,
}

/// Process context with exclusive access to global entities, parameters and
/// communication channels.
pub struct This {
    // Standard I/O
    display: con::Display,
    input: std::io::Stdin,
    output: std::io::Stdout,

    // Task properties
    workdir: std::path::PathBuf,
}

impl DisplayOption {
    /// Parse the display option from its string representation.
    ///
    /// If the string does not represent any valid display option, `None` is
    /// returned. Note that matching is case-sensitive.
    pub fn from_string(v: &str) -> Option<Self> {
        match v {
            "auto" => Some(DisplayOption::Auto),
            "plain" => Some(DisplayOption::Plain),
            "ansi" => Some(DisplayOption::Ansi),
            "wincon" => Some(DisplayOption::Wincon),
            _ => None,
        }
    }
}

impl This {
    fn with(
        display: con::Display,
        input: std::io::Stdin,
        output: std::io::Stdout,
        workdir: std::path::PathBuf,
    ) -> Self {
        Self {
            display: display,
            input: input,
            output: output,
            workdir: workdir,
        }
    }

    /// Create a new process context from ambient capabilities.
    ///
    /// This will query ambient capabilities of the process and create the
    /// process context from it. All information is copied at the time of this
    /// call and thus will represent the ambient capabilities of the process
    /// at this time. Later changes to the ambient capabilities of the process
    /// will (intentionally) not reflect into the context.
    ///
    /// This function will assume that ambient process capabilities are
    /// accessible. It will panic if not.
    pub fn from_ambient() -> Self {
        let v_display = ambient_display(None);
        let v_input = std::io::stdin();
        let v_output = std::io::stdout();
        let v_workdir = std::env::current_dir().expect("Current working directory must be set");

        Self::with(
            v_display,
            v_input,
            v_output,
            v_workdir,
        )
    }

    /// Yield access to the display abstraction.
    pub fn display(&mut self) -> &mut con::Display {
        &mut self.display
    }

    /// Yield access to the process-input abstraction.
    pub fn input(&mut self) -> &mut std::io::Stdin {
        &mut self.input
    }

    /// Yield access to the process-output abstraction.
    pub fn output(&mut self) -> &mut std::io::Stdout {
        &mut self.output
    }

    /// Yield access to the working directory.
    pub fn workdir(&self) -> &std::path::Path {
        &self.workdir
    }

    /// Update the display option and reinitialize the display handler.
    ///
    /// This might re-read ambient capabilities to re-initialize the display
    /// according to the newly selected options.
    pub fn set_display_option(&mut self, opt: Option<DisplayOption>) {
        self.display = ambient_display(opt);
    }
}

fn ambient_display(
    opt: Option<DisplayOption>,
) -> con::Display {
    let stream = std::io::stderr();

    #[allow(unused_mut)]
    let mut mode = None;

    // If the user selects a mode explicitly, we will always enforce that. Note
    // that incompatible selections will possibly lead to fatal errors and
    // abort the application. Use the auto-detection if adaptations are needed.
    mode = mode.or_else(|| {
        match opt {
            None => None,
            Some(DisplayOption::Auto) => None,
            Some(DisplayOption::Plain) => Some(con::Mode::Plain),
            Some(DisplayOption::Ansi) => Some(con::Mode::Ansi),
            Some(DisplayOption::Wincon) => Some(con::Mode::Wincon),
        }
    });

    // For windows devices we can use the Windows Console API to get a slightly
    // better display experience (mainly basic colors). Additionally, newer
    // Windows versions allow enabling ANSI-compatibility. Preferably, the
    // calling console would do that, so they could mediate between parallel
    // applications that run in the same console (e.g., background jobs in the
    // shell). Unfortunately, they do not. But they do reset console modes when
    // starting applications, so it should be fine if applications adjust the
    // console to their needs. This is also the Microsoft recommended approach.
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32;

        mode = mode.or_else(|| {
            let handle = std::os::windows::io::AsRawHandle::as_raw_handle(
                &stream,
            ) as isize;

            let mut cm: Win32::System::Console::CONSOLE_MODE = 0;

            // Query the console mode to get information on the supported
            // capabilities. If the console cannot be queried, then the display
            // device is not a console and we avoid treating it as such.
            {
                let r = unsafe {
                    Win32::System::Console::GetConsoleMode(handle, &mut cm) == 0
                };

                if !r {
                    return None;
                }
            }

            // If ANSI-compatibility is already enabled, we can make use of it
            // without requiring any modifications.
            if (cm & Win32::System::Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING)
                == Win32::System::Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING
            {
                return Some(con::Mode::Ansi);
            }

            // Try enabling the ANSI-compatibility. If successful, we can use
            // make use of it.
            {
                cm |= Win32::System::Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING;

                let r = unsafe {
                    Win32::System::Console::SetConsoleMode(handle, cm) == 0
                };

                if r {
                    return Some(con::Mode::Ansi);
                }
            }

            // ANSI-compatibility could not be enabled, so lets use the
            // less-capable Console API mode.
            Some(con::Mode::Wincon)
        });
    }

    // For POSIX systems, we use `IsTerminal` from the standard library, which
    // will run `isatty(3)` for us. If successful, we can run in ANSI mode.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        mode = mode.or_else(|| {
            if std::io::IsTerminal::is_terminal(&stream) {
                Some(con::Mode::Ansi)
            } else {
                None
            }
        });
    }

    // Create the display device with the selected mode (or plain as fallback).
    con::Display::with(
        stream,
        mode.unwrap_or(con::Mode::Plain),
    )
}
