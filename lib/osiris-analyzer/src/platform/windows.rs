//! Platform Layer: Windows
//!
//! Implement the application and UI via win32, using the classic window-based
//! UI handling.
//!
//! The UI uses a simple text-view to show all output, and an input-entry
//! to accept commands from the user.

use windows::{self, Win32};

const APP_CLASS: windows::core::PCSTR = windows::core::s!("foo.osiris.demo.root");
const APP_TITLE: windows::core::PCSTR = windows::core::s!("Osiris Demo Application");

struct Control {
    window: Win32::Foundation::HWND,
    log: Win32::Foundation::HWND,
    input: Win32::Foundation::HWND,
    edit_height: i32,
}

struct Window {
    module: Win32::Foundation::HMODULE,
    cwnd: u16,
    hwnd: core::cell::Cell<Option<Win32::Foundation::HWND>>,
    ctrl: core::cell::RefCell<Option<Control>>,
}

type WindowRef = core::pin::Pin<std::rc::Rc<Window>>;

pub struct App {
    window: WindowRef,
}

// This is missing from `windows-rs`, see MSDN for details. Turns an ATOM into
// a PCSTR (simply stores the ATOM in the lower word and sets the higher word
// to 0).
#[allow(non_snake_case)]
fn MAKEINTATOM(atom: u16) -> windows::core::PCSTR {
    windows::core::PCSTR::from_raw(atom as usize as *const u8)
}

fn gdi_textmetric(
    window: Win32::Foundation::HWND,
    font: Win32::Graphics::Gdi::HGDIOBJ,
) -> Win32::Graphics::Gdi::TEXTMETRICA {
    unsafe {
        let hdc = Win32::Graphics::Gdi::GetDC(window);
        let prev = Win32::Graphics::Gdi::SelectObject(hdc, font);

        let mut tm: Win32::Graphics::Gdi::TEXTMETRICA = Default::default();
        Win32::Graphics::Gdi::GetTextMetricsA(hdc, &mut tm);

        Win32::Graphics::Gdi::SelectObject(hdc, prev);
        Win32::Graphics::Gdi::ReleaseDC(window, hdc);

        tm
    }
}

impl Control {
    fn layout(
        window: Win32::Foundation::HWND,
        edit_height: i32,
    ) -> (
        Win32::Foundation::RECT,
        Win32::Foundation::RECT,
    ) {
        let mut dim_window: Win32::Foundation::RECT = Default::default();
        let mut dim_log: Win32::Foundation::RECT = Default::default();
        let mut dim_input: Win32::Foundation::RECT = Default::default();

        unsafe {
            windows::Win32::UI::WindowsAndMessaging::GetClientRect(
                window,
                &mut dim_window,
            ).expect("error: cannot query window dimensions");
        }

        dim_input.left = 0;
        dim_input.top = dim_window.bottom - edit_height;
        dim_input.right = dim_window.right;
        dim_input.bottom = dim_window.bottom;

        dim_log.left = 0;
        dim_log.top = 0;
        dim_log.right = dim_window.right;
        dim_log.bottom = dim_input.top;

        (dim_log, dim_input)
    }

    pub fn new(
        module: Win32::Foundation::HMODULE,
        window: Win32::Foundation::HWND,
    ) -> Self {
        // Use a default `EDIT`-height of 24 to calculate the initial
        // layout. We cannot reasonably calculate the borders of the
        // `EDIT` control before creating it, so we go with a default.
        let (dim_log, dim_input) = Self::layout(window, 24);

        // Create log window as a multiline `EDIT` control.
        let log = unsafe {
            Win32::UI::WindowsAndMessaging::CreateWindowExA(
                Win32::UI::WindowsAndMessaging::WS_EX_CLIENTEDGE,
                windows::core::s!("EDIT"),
                windows::core::s!(""),
                Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                    Win32::UI::WindowsAndMessaging::ES_AUTOHSCROLL as u32,
                )
                    | Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                        Win32::UI::WindowsAndMessaging::ES_AUTOVSCROLL as u32,
                    )
                    | Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                        Win32::UI::WindowsAndMessaging::ES_MULTILINE as u32,
                    )
                    | Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                        Win32::UI::WindowsAndMessaging::ES_READONLY as u32,
                    )
                    | Win32::UI::WindowsAndMessaging::WS_CHILD
                    | Win32::UI::WindowsAndMessaging::WS_HSCROLL
                    | Win32::UI::WindowsAndMessaging::WS_VISIBLE
                    | Win32::UI::WindowsAndMessaging::WS_VSCROLL,
                dim_log.left,
                dim_log.top,
                dim_log.right - dim_log.left,
                dim_log.bottom - dim_log.top,
                window,
                None,
                module,
                None,
            )
        };
        assert_ne!(log, Win32::Foundation::HWND(0), "error: cannot create log control");

        // Create input window as a single-line `EDIT` control.
        let input = unsafe {
            Win32::UI::WindowsAndMessaging::CreateWindowExA(
                Win32::UI::WindowsAndMessaging::WS_EX_CLIENTEDGE,
                windows::core::s!("EDIT"),
                windows::core::s!(""),
                Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                    Win32::UI::WindowsAndMessaging::ES_AUTOHSCROLL as u32,
                )
                    | Win32::UI::WindowsAndMessaging::WS_CHILD
                    | Win32::UI::WindowsAndMessaging::WS_VISIBLE,
                dim_input.left,
                dim_input.top,
                dim_input.right - dim_input.left,
                dim_input.bottom - dim_input.top,
                window,
                None,
                module,
                None,
            )
        };
        assert_ne!(input, Win32::Foundation::HWND(0), "error: cannot create input control");

        // Move focus to the input control.
        unsafe {
            Win32::UI::Input::KeyboardAndMouse::SetFocus(input);
        }

        // Configure the default GUI font for all controls.
        let font = unsafe {
            let font = Win32::Graphics::Gdi::GetStockObject(
                Win32::Graphics::Gdi::DEFAULT_GUI_FONT,
            );

            Win32::UI::WindowsAndMessaging::SendMessageA(
                log,
                Win32::UI::WindowsAndMessaging::WM_SETFONT,
                Win32::Foundation::WPARAM(font.0 as usize),
                Win32::Foundation::LPARAM(0),
            );
            Win32::UI::WindowsAndMessaging::SendMessageA(
                input,
                Win32::UI::WindowsAndMessaging::WM_SETFONT,
                Win32::Foundation::WPARAM(font.0 as usize),
                Win32::Foundation::LPARAM(0),
            );

            font
        };

        // Calculate standard height of the input control.
        let edit_height = unsafe {
            // Query the text-metrics of the input control.
            let tm = gdi_textmetric(input, font);

            // Calculate the border-size of the input control.
            let mut dim_window: Win32::Foundation::RECT = Default::default();
            let mut dim_client: Win32::Foundation::RECT = Default::default();
            windows::Win32::UI::WindowsAndMessaging::GetWindowRect(
                input,
                &mut dim_window,
            ).expect("error: cannot query window dimensions");
            windows::Win32::UI::WindowsAndMessaging::GetClientRect(
                input,
                &mut dim_client,
            ).expect("error: cannot query window dimensions");
            let border = (dim_window.bottom - dim_window.top) - (dim_client.bottom - dim_client.top);

            // Set the height of an edit-control to the font-height plus the
            // border plus 10% padding of the font (but minimum 2px).
            tm.tmHeight + border + core::cmp::max(2, tm.tmHeight * 10 / 100)
        };

        Self {
            window: window,
            log: log,
            input: input,
            edit_height: edit_height,
        }
    }

    pub fn resize(&self) {
        let (dim_log, dim_input) = Self::layout(self.window, self.edit_height);

        unsafe {
            windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                self.log,
                None,
                dim_log.left,
                dim_log.top,
                dim_log.right - dim_log.left,
                dim_log.bottom - dim_log.top,
                Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
            ).expect("error: cannot update control dimensions");

            windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                self.input,
                None,
                dim_input.left,
                dim_input.top,
                dim_input.right - dim_input.left,
                dim_input.bottom - dim_input.top,
                Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
            ).expect("error: cannot update control dimensions");
        }
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        unsafe {
            let _ = Win32::UI::WindowsAndMessaging::DestroyWindow(self.input);
            let _ = Win32::UI::WindowsAndMessaging::DestroyWindow(self.log);
        }
    }
}

impl Window {
    pub fn new(
        module: Win32::Foundation::HMODULE,
        window_class: windows::core::PCSTR,
        window_title: windows::core::PCSTR,
    ) -> WindowRef {
        // Register a window-class for the root-level window.
        let cwnd: u16;
        unsafe {
            let wc = Win32::UI::WindowsAndMessaging::WNDCLASSA {
                hCursor: Win32::UI::WindowsAndMessaging::LoadCursorW(
                    None,
                    Win32::UI::WindowsAndMessaging::IDC_ARROW,
                ).expect("error: cannot load arrow cursor"),

                hbrBackground: Win32::Graphics::Gdi::HBRUSH(
                    Win32::Graphics::Gdi::COLOR_WINDOW.0 as isize,
                ),

                hInstance: module.into(),
                lpfnWndProc: Some(Self::wndproc),
                lpszClassName: window_class,

                style: Win32::UI::WindowsAndMessaging::CS_HREDRAW
                    | Win32::UI::WindowsAndMessaging::CS_VREDRAW,

                ..Default::default()
            };

            cwnd = Win32::UI::WindowsAndMessaging::RegisterClassA(&wc);
            assert_ne!(cwnd, 0, "error: cannot register window class");
        }

        // Create the partial window with an initial reference.
        let window_ref: WindowRef = std::rc::Rc::pin(
            Self {
                module: module,
                cwnd: cwnd,
                hwnd: Default::default(),
                ctrl: Default::default(),
            }
        );

        // Create the root-level window.
        let hwnd;
        unsafe {
            hwnd = Win32::UI::WindowsAndMessaging::CreateWindowExA(
                Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                MAKEINTATOM(cwnd),
                window_title,
                Win32::UI::WindowsAndMessaging::WS_OVERLAPPEDWINDOW,
                Win32::UI::WindowsAndMessaging::CW_USEDEFAULT,
                Win32::UI::WindowsAndMessaging::CW_USEDEFAULT,
                Win32::UI::WindowsAndMessaging::CW_USEDEFAULT,
                Win32::UI::WindowsAndMessaging::CW_USEDEFAULT,
                None,
                None,
                module,
                Some(
                    core::mem::transmute::<_, *const core::ffi::c_void>(
                        window_ref.clone(),
                    ),
                ),
            );
            assert_ne!(hwnd, Win32::Foundation::HWND(0), "error: cannot create root window");
        }
        window_ref.hwnd.set(Some(hwnd));

        window_ref
    }

    pub fn show(&self) {
        unsafe {
            Win32::UI::WindowsAndMessaging::ShowWindow(
                self.hwnd.get().unwrap(),
                Win32::UI::WindowsAndMessaging::SW_SHOWDEFAULT,
            );
        }
    }

    fn ctrl(&self) -> core::cell::Ref<'_, Control> {
        core::cell::Ref::map(self.ctrl.borrow(), |v| v.as_ref().unwrap())
    }

    fn wndproc_self(
        self: core::pin::Pin<std::rc::Rc<Self>>,
        window: Win32::Foundation::HWND,
        message: u32,
        wparam: Win32::Foundation::WPARAM,
        lparam: Win32::Foundation::LPARAM,
    ) -> Win32::Foundation::LRESULT {
        match message {
            Win32::UI::WindowsAndMessaging::WM_CREATE => {
                let prev = self.ctrl.replace(
                    Some(Control::new(self.module, window)),
                );
                assert!(prev.is_none());

                Win32::Foundation::LRESULT(0)
            },

            Win32::UI::WindowsAndMessaging::WM_DESTROY => {
                let prev = self.ctrl.replace(None);
                drop(prev);

                unsafe {
                    Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
                }

                Win32::Foundation::LRESULT(0)
            },

            Win32::UI::WindowsAndMessaging::WM_SIZE => {
                self.ctrl().resize();

                Win32::Foundation::LRESULT(0)
            },

            _ => {
                unsafe {
                    Win32::UI::WindowsAndMessaging::DefWindowProcA(
                        window,
                        message,
                        wparam,
                        lparam,
                    )
                }
            },
        }
    }

    extern "system" fn wndproc(
        window: Win32::Foundation::HWND,
        message: u32,
        wparam: Win32::Foundation::WPARAM,
        lparam: Win32::Foundation::LPARAM,
    ) -> Win32::Foundation::LRESULT {
        // This `wndproc` callback might be invoked by anyone with access to
        // the `HWND`, and thus there is no owning context that ensures the
        // `Window` object is valid. Instead, we store a reference to it in
        // `HWND` and acquire it in every `wndproc` call. We then dispatch
        // messages to the `Window` object, and allow destruction of the `HWND`
        // even. Since the `wndproc` holds a reference, this is safe.

        let window_ref: Option<WindowRef> = match message {
            Win32::UI::WindowsAndMessaging::WM_NCCREATE => {
                // Steal the reference from `CreateParams` and move it into
                // `window_ref` for this execution of `wndproc`. Then acquire
                // another reference and store it in `GWL_USERDATA` for future
                // callbacks. It is released in `WM_NCDESTROY`.
                let uargs = lparam.0 as *const Win32::UI::WindowsAndMessaging::CREATESTRUCTA;
                assert!(!uargs.is_null());

                let args = unsafe { &*uargs };
                assert!(!args.lpCreateParams.is_null());

                let wref = unsafe {
                    core::mem::transmute::<_, WindowRef>(args.lpCreateParams)
                };

                unsafe {
                    Win32::UI::WindowsAndMessaging::SetWindowLongPtrA(
                        window,
                        Win32::UI::WindowsAndMessaging::GWL_USERDATA,
                        core::mem::transmute::<_, *const core::ffi::c_void>(
                            wref.clone(),
                        ) as isize,
                    );
                }

                Some(wref)
            },
            Win32::UI::WindowsAndMessaging::WM_NCDESTROY => {
                // Clear GWL_USERDATA to 0, but take ownership of the previous
                // value, which is a window reference. We then grant this
                // window reference to `wndproc`, which will drop it before
                // returning.
                let uptr = unsafe {
                    Win32::UI::WindowsAndMessaging::SetWindowLongPtrA(
                        window,
                        Win32::UI::WindowsAndMessaging::GWL_USERDATA,
                        0,
                    ) as *mut core::ffi::c_void
                };
                assert!(!uptr.is_null());

                unsafe {
                    Some(core::mem::transmute::<_, WindowRef>(uptr))
                }
            },
            _ => {
                // Acquire a reference to the `Window` object from
                // `GWL_USERDATA` for this execution of `wndproc`. Ensure
                // that we do not steal the reference from `GWL_USERDATA`,
                // but leave it around for the next callback.
                let uptr = unsafe {
                    Win32::UI::WindowsAndMessaging::GetWindowLongPtrA(
                        window,
                        Win32::UI::WindowsAndMessaging::GWL_USERDATA,
                    ) as *mut core::ffi::c_void
                };
                if uptr.is_null() {
                    return Win32::Foundation::LRESULT(0);
                }

                let wref = unsafe {
                    core::mem::transmute::<_, WindowRef>(uptr)
                };

                core::mem::forget(wref.clone());

                Some(wref)
            }
        };

        match window_ref {
            Some(wref) => {
                wref.wndproc_self(
                    window,
                    message,
                    wparam,
                    lparam,
                )
            },
            None => {
                unsafe {
                    Win32::UI::WindowsAndMessaging::DefWindowProcA(
                        window,
                        message,
                        wparam,
                        lparam,
                    )
                }
            },
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if let Some(ctrl) = self.ctrl.replace(None) {
                drop(ctrl);
            }
            if let Some(hwnd) = self.hwnd.get() {
                let _ = Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
            }
            let _ = Win32::UI::WindowsAndMessaging::UnregisterClassA(
                MAKEINTATOM(self.cwnd),
                self.module,
            );
        }
    }
}

impl App {
    pub fn new() -> Self {
        let module = unsafe {
            Win32::System::LibraryLoader::GetModuleHandleA(None)
        }.expect("error: cannot acquire own module handle");

        Self {
            window: Window::new(module, APP_CLASS, APP_TITLE),
        }
    }

    pub fn run(&self) -> std::process::ExitCode {
        let mut message = Win32::UI::WindowsAndMessaging::MSG::default();

        self.window.show();

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
            unsafe {
                Win32::UI::WindowsAndMessaging::TranslateMessage(&message);
                Win32::UI::WindowsAndMessaging::DispatchMessageA(&message);
            }
        }

        0.into()
    }
}
