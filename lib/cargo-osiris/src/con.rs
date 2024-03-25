//! # Console Handling
//!
//! The console module provides abstractions to interact with a user via
//! a visual console. It supports different operating system abstractions,
//! but falls back to plain character streams if required.

enum Color {
    Red,
    Yellow,
    Blue,
    Cyan,
}

#[derive(Default)]
struct Style {
    color_fg: Option<Color>,
    color_bg: Option<Color>,
    bold: bool,
}

enum Device {
    Plain {
        stream: std::io::Stderr,
    },
    Ansi {
        stream: std::io::Stderr,
    },
    Wincon {
        stream: std::io::Stderr,
        attr_fg: u16,
        attr_bg: u16,
    },
}

/// Console display mode to run with
pub enum Mode {
    /// Plain output without any graphical adaptations
    Plain,
    /// Use ANSI escape sequences for graphical output
    Ansi,
    /// Use Windows-console APIs for graphical output
    Wincon,
}

/// Console display device for plain and graphical output. Depending on the
/// selected mode, this will either output plain messages or graphically
/// enhance them.
///
/// This display device will run in strict mode and panic if the underlying
/// device fails.
pub struct Display {
    device: Device,
}

impl Device {
    fn new_plain(stream: std::io::Stderr) -> Self {
        Self::Plain { stream: stream }
    }

    fn new_ansi(stream: std::io::Stderr) -> Self {
        Self::Ansi { stream: stream }
    }

    fn new_wincon(stream: std::io::Stderr) -> Self {
        #[cfg(not(target_os = "windows"))]
        {
            Self::Wincon {
                stream: stream,
                attr_fg: 0,
                attr_bg: 0,
            }
        }

        #[cfg(target_os = "windows")]
        {
            use windows_sys::Win32;

            // Query the display console for the current character attributes. We
            // remember them as default attributes, since the windows console does
            // not support restoring defaults or previous colors.

            let handle = std::os::windows::io::AsRawHandle::as_raw_handle(
                &stream,
            ) as isize;

            let mut info = Win32::System::Console::CONSOLE_SCREEN_BUFFER_INFO {
                dwSize: Win32::System::Console::COORD { X: 0, Y: 0 },
                dwCursorPosition: Win32::System::Console::COORD { X: 0, Y: 0 },
                wAttributes: 0,
                srWindow: Win32::System::Console::SMALL_RECT { Left: 0, Top: 0, Right: 0, Bottom: 0 },
                dwMaximumWindowSize: Win32::System::Console::COORD { X: 0, Y: 0 },
            };

            unsafe {
                (Win32::System::Console::GetConsoleScreenBufferInfo(
                    handle,
                    &mut info,
                ) == 0)
                    .then_some(())
                    .expect("display console must be introspectable")
            }

            Self::Wincon {
                stream: stream,
                attr_fg: (info.wAttributes & 0x0f),
                attr_bg: (info.wAttributes & 0xf0),
            }
        }
    }

    fn write_plain(
        &mut self,
        data: &dyn core::fmt::Display,
    ) {
        match self {
            Device::Plain { stream }
            | Device::Ansi { stream }
            | Device::Wincon { stream, .. } => {
                std::io::Write::write_fmt(
                    stream,
                    core::format_args!("{}", data),
                ).expect("display console must be writable");
            },
        }
    }

    fn write_styled(
        &mut self,
        data: &dyn core::fmt::Display,
        style: &Style,
    ) {
        match self {
            Device::Plain { .. } => {
                self.write_plain(data);
            },

            Device::Ansi { stream } => {
                let maybe_fg = match &style.color_fg {
                    None => "",
                    Some(Color::Red) => ";31",
                    Some(Color::Yellow) => ";33",
                    Some(Color::Blue) => ";34",
                    Some(Color::Cyan) => ";36",
                };
                let maybe_bg = match &style.color_bg {
                    None => "",
                    Some(Color::Red) => ";41",
                    Some(Color::Yellow) => ";43",
                    Some(Color::Blue) => ";44",
                    Some(Color::Cyan) => ";46",
                };
                let maybe_bold = match style.bold {
                    false => "",
                    true => ";1",
                };

                std::io::Write::write_fmt(
                    stream,
                    core::format_args!(
                        core::concat!(
                            "\x1b[{}{}{}m",     // SGR clear + codes
                            "{}",
                            "\x1b[m",           // SGR clear
                        ),
                        maybe_fg,
                        maybe_bg,
                        maybe_bold,
                        data,
                    ),
                ).expect("display console must be writable");
            },

            #[cfg(not(target_os = "windows"))]
            Device::Wincon { .. } => {
                self.write_plain(data);
            },

            #[cfg(target_os = "windows")]
            Device::Wincon {
                stream,
                attr_fg,
                attr_bg,
                ..
            } => {
                use windows_sys::Win32;

                let handle = std::os::windows::io::AsRawHandle::as_raw_handle(
                    stream,
                ) as isize;

                let fg = match &style.color_fg {
                    None => *attr_fg,
                    Some(Color::Red) => Win32::System::Console::FOREGROUND_RED,
                    Some(Color::Yellow) => {
                        Win32::System::Console::FOREGROUND_GREEN
                        | Win32::System::Console::FOREGROUND_RED
                    },
                    Some(Color::Blue) => Win32::System::Console::FOREGROUND_BLUE,
                    Some(Color::Cyan) => {
                        Win32::System::Console::FOREGROUND_BLUE
                        | Win32::System::Console::FOREGROUND_GREEN
                    },
                };

                let bg = match &style.color_bg {
                    None => *attr_bg,
                    Some(Color::Red) => Win32::System::Console::BACKGROUND_RED,
                    Some(Color::Yellow) => {
                        Win32::System::Console::BACKGROUND_GREEN
                        | Win32::System::Console::BACKGROUND_RED
                    },
                    Some(Color::Blue) => Win32::System::Console::BACKGROUND_BLUE,
                    Some(Color::Cyan) => {
                        Win32::System::Console::BACKGROUND_BLUE
                        | Win32::System::Console::BACKGROUND_GREEN
                    },
                };

                // Flush the output buffer to ensure previous data is not
                // affected by the attribute change. Then set the new
                // attributes and write the requested data. Flush again to
                // ensure all data is written with the new attributes, before
                // resetting the attributes to the default value.

                std::io::Write::flush(stream)
                    .expect("display console must be writable");

                unsafe {
                    (Win32::System::Console::SetConsoleTextAttribute(
                        handle,
                        fg | bg,
                    ) == 0)
                        .then_some(())
                        .expect("display console must be writable");
                }

                std::io::Write::write_fmt(
                    stream,
                    core::format_args!("{}", data),
                ).expect("display console must be writable");

                std::io::Write::flush(stream)
                    .expect("display console must be writable");

                unsafe {
                    (Win32::System::Console::SetConsoleTextAttribute(
                        handle,
                        *attr_fg | *attr_bg,
                    ) == 0)
                        .then_some(())
                        .expect("display console must be writable");
                }
            },
        }
    }

    fn status(
        &mut self,
        status: &dyn core::fmt::Display,
        message: Option<&dyn core::fmt::Display>,
    ) {
        // Status messages are designed to match `Cargo`. Hence, they use 12ch
        // right-aligned status headers followed by the status message. The
        // colors are different to better distinguish the messages from Cargo.

        self.write_styled(
            &core::format_args!("{:>12}", status),
            &Style { color_fg: Some(Color::Cyan), bold: true, ..Default::default() },
        );
        self.write_plain(&core::format_args!(" {}\n", message.unwrap_or(&"")));
    }

    fn warning(
        &mut self,
        message: &dyn core::fmt::Display,
    ) {
        self.write_styled(
            &"warning",
            &Style { color_fg: Some(Color::Yellow), bold: true, ..Default::default() },
        );
        self.write_styled(&":", &Style { bold: true, ..Default::default() });
        self.write_plain(&core::format_args!(" {}\n", message));
    }

    fn error(
        &mut self,
        message: &dyn core::fmt::Display,
    ) {
        self.write_styled(
            &"error",
            &Style { color_fg: Some(Color::Red), bold: true, ..Default::default() },
        );
        self.write_styled(&":", &Style { bold: true, ..Default::default() });
        self.write_plain(&core::format_args!(" {}\n", message));
    }
}

impl Display {
    /// Create a new display device with the selected stream and
    /// mode configuration.
    pub fn with(
        stream: std::io::Stderr,
        mode: Mode,
    ) -> Self {
        let device = match mode {
            Mode::Plain => Device::new_plain(stream),
            Mode::Ansi => Device::new_ansi(stream),
            Mode::Wincon => Device::new_wincon(stream),
        };

        Self {
            device: device,
        }
    }

    /// Display a status message with the indicated status as well as,
    /// optionally, a following message.
    pub fn status(
        &mut self,
        status: &dyn core::fmt::Display,
        message: Option<&dyn core::fmt::Display>,
    ) {
        self.device.status(status, message)
    }

    /// Print a warning message.
    pub fn warning(
        &mut self,
        message: &dyn core::fmt::Display,
    ) {
        self.device.warning(message)
    }

    /// Print an error message.
    pub fn error(
        &mut self,
        message: &dyn core::fmt::Display,
    ) {
        self.device.error(message)
    }
}
