//! # Console Handling
//!
//! The console module provides abstractions to interact with a user via
//! a visual console. It supports different operating system abstractions,
//! but falls back to plain character streams if required.

use crate::misc;

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
        width: usize,
    },
    Wincon {
        stream: std::io::Stderr,
        width: usize,
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
        #[cfg(not(all(target_os = "linux", target_env = "gnu")))]
        {
            Self::Ansi {
                stream: stream,
                width: 80,
            }
        }

        #[cfg(all(target_os = "linux", target_env = "gnu"))]
        {
            // Query the display console for window dimensions and remember the
            // width. This is used to ellipse text and adapt to horizontal
            // restrictions.
            // Note that we do not listen for `SIGWINCH` but expect the
            // dimensions to be static for the lifetime of this module.

            const TIOCGWINSZ: usize = 0x5413;

            #[repr(C)]
            struct winsize {
                pub ws_row: u16,
                pub ws_col: u16,
                pub ws_xpixel: u16,
                pub ws_ypixel: u16,
            }

            extern "C" {
                fn ioctl(fd: u32, io: usize, ...) -> u32;
            }

            let fd = std::os::fd::AsFd::as_fd(&stream);

            let mut info = winsize {
                ws_row: 0,
                ws_col: 0,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };

            unsafe {
                (ioctl(core::mem::transmute(fd), TIOCGWINSZ, &mut info) == 0)
                    .then_some(())
                    .expect("display console must be introspectable");
            }

            Self::Ansi {
                stream: stream,
                width: info.ws_col as usize,
            }
        }
    }

    fn new_wincon(stream: std::io::Stderr) -> Self {
        #[cfg(not(target_os = "windows"))]
        {
            Self::Wincon {
                stream: stream,
                width: 80,
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
                width: 80,
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
            Device::Plain { stream, .. }
            | Device::Ansi { stream, .. }
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

            Device::Ansi { stream, .. } => {
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

    fn annotation(
        &mut self,
        lines_head: Option<&mut dyn Iterator<Item = &dyn core::fmt::Display>>,
        line: &dyn core::fmt::Display,
        line_number: Option<usize>,
        lines_tail: Option<&mut dyn Iterator<Item = &dyn core::fmt::Display>>,
        range: core::ops::Range<usize>,
    ) {
        let str_line = format!("{}", line);
        let n_line = str_line.len();
        let ellipsis = "[..]";
        let n_ellipsis = ellipsis.len();
        let str_lno_prefix = match line_number {
            None => None,
            Some(v) => Some(format!("{} | ", v)),
        };
        let n_lno_prefix = str_lno_prefix
            .as_ref()
            .map(|v| v.len())
            .unwrap_or(0);
        let o_width = match self {
            Device::Plain { .. } => None,
            Device::Ansi { width, .. }
            | Device::Wincon { width, .. } => Some(*width),
        };

        // Print leading empty line
        self.write_styled(
            &core::format_args!(
                "{0:>1$.2$}\n",
                "| ", n_lno_prefix, n_lno_prefix,
            ),
            &Style { color_fg: Some(Color::Blue), bold: true, ..Default::default() },
        );

        // Print leading lines
        if let Some(list) = lines_head {
            for v in list {
                self.write_styled(
                    &core::format_args!(
                        "{0:>1$.2$}",
                        "| ", n_lno_prefix, n_lno_prefix,
                    ),
                    &Style { color_fg: Some(Color::Blue), bold: true, ..Default::default() },
                );
                self.write_plain(&core::format_args!("{}\n", v));
            }
        }

        // Print the referenced line and the annotation marker
        {
            let mut ellipsed = match o_width {
                None => (
                    core::ops::Range { start: 0, end: n_line },
                    core::ops::Range { start: n_line, end: n_line },
                ),
                Some(width) => misc::ellipse(
                    &str_line,
                    range.clone(),
                    width.saturating_sub(n_lno_prefix),
                ),
            };

            // Make room for the ellipses. If the individual zones are too
            // small to display the ellipsis, this will lead to overlong
            // lines. This seems acceptable.
            let (ellipsis_left, n_ellipsis_left) = if ellipsed.0.start > 0 {
                ellipsed.0.start = ellipsed.0.start.saturating_add(n_ellipsis);
                (ellipsis, n_ellipsis)
            } else {
                ("", 0)
            };
            let (ellipsis_right, _n_ellipsis_right) = if ellipsed.1.end < n_line {
                ellipsed.1.end = ellipsed.1.end.saturating_sub(n_ellipsis);
                (ellipsis, n_ellipsis)
            } else {
                ("", 0)
            };
            let (ellipsis_center, n_ellipsis_center) = if ellipsed.0.end < ellipsed.1.start {
                ellipsed.1.start = ellipsed.1.start.saturating_add(n_ellipsis);
                (ellipsis, n_ellipsis)
            } else {
                ("", 0)
            };

            self.write_styled(
                &core::format_args!(
                    "{}{}",
                    str_lno_prefix.as_deref().unwrap_or(""),
                    ellipsis_left,
                ),
                &Style { color_fg: Some(Color::Blue), bold: true, ..Default::default() },
            );
            self.write_plain(&&str_line[ellipsed.0.clone()]);
            self.write_styled(
                &core::format_args!("{}", ellipsis_center),
                &Style { color_fg: Some(Color::Blue), bold: true, ..Default::default() },
            );
            self.write_plain(&&str_line[ellipsed.1.clone()]);
            self.write_styled(
                &core::format_args!("{}\n", ellipsis_right),
                &Style { color_fg: Some(Color::Blue), bold: true, ..Default::default() },
            );

            let hl_shift = range.start.saturating_sub(ellipsed.0.start) + n_ellipsis_left;
            let hl_l1 = ellipsed.0.end.saturating_sub(range.start);
            let hl_l2 = range.end.saturating_sub(ellipsed.1.start);

            self.write_styled(
                &core::format_args!(
                    "{0:>1$.2$}{3:>4$}{5:^>6$}{7:~>8$}{9:^>10$}\n",
                    "| ", n_lno_prefix, n_lno_prefix,
                    "", hl_shift,
                    "", hl_l1,
                    "", n_ellipsis_center,
                    "", hl_l2,
                ),
                &Style { color_fg: Some(Color::Blue), bold: true, ..Default::default() },
            );
        }

        // Print trailing lines
        if let Some(list) = lines_tail {
            for v in list {
                self.write_styled(
                    &core::format_args!(
                        "{0:>1$.2$}",
                        "| ", n_lno_prefix, n_lno_prefix,
                    ),
                    &Style { color_fg: Some(Color::Blue), bold: true, ..Default::default() },
                );
                self.write_plain(&core::format_args!("{}\n", v));
            }
        }

        // Print trailing empty line
        self.write_styled(
            &core::format_args!(
                "{0:>1$.2$}\n",
                "| ", n_lno_prefix, n_lno_prefix,
            ),
            &Style { color_fg: Some(Color::Blue), bold: true, ..Default::default() },
        );
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

    /// Print annotated lines.
    pub fn annotation<
        'head,
        'tail,
        HeadList,
        HeadItem,
        TailList,
        TailItem,
    >(
        &mut self,
        lines_head: Option<HeadList>,
        line: &dyn core::fmt::Display,
        line_number: Option<usize>,
        lines_tail: Option<TailList>,
        range: core::ops::Range<usize>,
    )
    where
        HeadList: IntoIterator<Item = &'head HeadItem>,
        HeadItem: 'head + core::fmt::Display,
        TailList: IntoIterator<Item = &'tail TailItem>,
        TailItem: 'tail + core::fmt::Display,
    {
        self.device.annotation(
            lines_head
                .map(|v| v.into_iter().map(|v| -> &dyn core::fmt::Display { v }))
                .as_mut()
                .map(|v| -> &mut dyn Iterator<Item = _> { v }),
            line,
            line_number,
            lines_tail
                .map(|v| v.into_iter().map(|v| -> &dyn core::fmt::Display { v }))
                .as_mut()
                .map(|v| -> &mut dyn Iterator<Item = _> { v }),
            range,
        )
    }
}
