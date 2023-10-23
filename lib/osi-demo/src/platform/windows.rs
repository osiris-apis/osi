//! Platform Layer: Windows
//!
//! Implement the application and UI via win32, using the classic window-based
//! UI handling.
//!
//! The UI uses a simple text-view to show all output, and an input-entry
//! to accept commands from the user.

use windows::{self, Win32};

const APP_CLASS_ROOT: windows::core::PCSTR = windows::core::s!("foo.osiris.demo.root");
const APP_NAME_ROOT: windows::core::PCSTR = windows::core::s!("Osiris Demo Application");

pub struct App {
    module: Win32::Foundation::HMODULE,
    root_class: u16,
    log_window: Win32::Foundation::HWND,
}

impl App {
    // This is missing from `windows-rs`, see MSDN for details. Turns an atom
    // into a PCSTR (simply stores the atom in the lower word and sets the
    // higher word to 0).
    #[allow(non_snake_case)]
    fn MAKEINTATOM(atom: u16) -> windows::core::PCSTR {
        windows::core::PCSTR::from_raw(atom as usize as *const u8)
    }

    extern "system" fn wndproc(
        window: Win32::Foundation::HWND,
        message: u32,
        wparam: Win32::Foundation::WPARAM,
        lparam: Win32::Foundation::LPARAM,
    ) -> Win32::Foundation::LRESULT {
        unsafe {
            match message {
                Win32::UI::WindowsAndMessaging::WM_CREATE => {
                    Win32::Foundation::LRESULT(0)
                },
                Win32::UI::WindowsAndMessaging::WM_DESTROY => {
                    Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
                    Win32::Foundation::LRESULT(0)
                },
                _ => Win32::UI::WindowsAndMessaging::DefWindowProcA(
                    window,
                    message,
                    wparam,
                    lparam,
                ),
            }
        }
    }

    fn build_class(module: Win32::Foundation::HMODULE) -> windows::core::Result<u16> {
        unsafe {
            let wc = Win32::UI::WindowsAndMessaging::WNDCLASSA {
                hCursor: Win32::UI::WindowsAndMessaging::LoadCursorW(None, Win32::UI::WindowsAndMessaging::IDC_ARROW)?,
                hbrBackground: Win32::Graphics::Gdi::HBRUSH(Win32::Graphics::Gdi::COLOR_WINDOW.0 as isize),
                hInstance: module.into(),
                lpfnWndProc: Some(Self::wndproc),
                lpszClassName: APP_CLASS_ROOT,
                style: Win32::UI::WindowsAndMessaging::CS_HREDRAW | Win32::UI::WindowsAndMessaging::CS_VREDRAW,
                ..Default::default()
            };

            match Win32::UI::WindowsAndMessaging::RegisterClassA(&wc) {
                0 => Err(windows::core::Error::from_win32()),
                v => Ok(v),
            }
        }
    }

    fn build_window(
        module: Win32::Foundation::HMODULE,
        class: u16,
    ) -> windows::core::Result<(
        Win32::Foundation::HWND,
    )> {
        unsafe {
            let root = match Win32::UI::WindowsAndMessaging::CreateWindowExA(
                Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                Self::MAKEINTATOM(class),
                APP_NAME_ROOT,
                Win32::UI::WindowsAndMessaging::WS_OVERLAPPEDWINDOW | Win32::UI::WindowsAndMessaging::WS_VISIBLE,
                Win32::UI::WindowsAndMessaging::CW_USEDEFAULT,
                Win32::UI::WindowsAndMessaging::CW_USEDEFAULT,
                Win32::UI::WindowsAndMessaging::CW_USEDEFAULT,
                Win32::UI::WindowsAndMessaging::CW_USEDEFAULT,
                None,
                None,
                module,
                None,
            ) {
                Win32::Foundation::HWND(0) => Err(windows::core::Error::from_win32()),
                v => Ok(v),
            }?;

            Ok((root,))
        }
    }

    pub fn new() -> Self {
        let module = unsafe {
            Win32::System::LibraryLoader::GetModuleHandleA(None)
        }.expect("error: cannot acquire own module handle");

        let class = Self::build_class(module).expect("error: cannot build root window class");
        let (log_wnd,) = Self::build_window(module, class).expect("error: cannot create root window");

        Self {
            module: module,
            root_class: class,
            log_window: log_wnd,
        }
    }

    pub fn run(&self) -> std::process::ExitCode {
        let mut message = Win32::UI::WindowsAndMessaging::MSG::default();

        while
            unsafe {
                Win32::UI::WindowsAndMessaging::GetMessageA(
                    &mut message,
                    None,
                    0,
                    0,
                ).into()
            }
        {
            unsafe { Win32::UI::WindowsAndMessaging::DispatchMessageA(&message) };
        }

        0.into()
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            let _ = Win32::UI::WindowsAndMessaging::DestroyWindow(
                self.log_window,
            );
            let _ = Win32::UI::WindowsAndMessaging::UnregisterClassA(
                App::MAKEINTATOM(self.root_class),
                self.module,
            );
        }
    }
}
