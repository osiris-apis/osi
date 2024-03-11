//! Platform Layer: macOS
//!
//! Implement the application and UI via Cocoa, using the classic NSWindow UI
//! handling.
//!
//! The UI uses a simple text-view to show all output, and an input-entry
//! to accept commands from the user.

use icrate;
use objc2;

#[derive(Default)]
pub struct AppDelegateVar {
    _align: u64,
    window: Option<objc2::rc::Id<icrate::AppKit::NSWindow>>,
}

unsafe impl objc2::encode::Encode for AppDelegateVar {
    const ENCODING: objc2::encode::Encoding = {
        objc2::encode::Encoding::Array(
            core::mem::size_of::<Self>() as u64,
            &<u64 as objc2::encode::Encode>::ENCODING,
        )
    };
}

#[repr(C)]
pub struct AppDelegate {
    // Base class must be at offset 0.
    base: objc2::runtime::NSObject,
}

// XXX: Requires rustc-1.77
#[cfg(any())]
const _: () = assert!(core::mem::offset_of!(AppDelegate, base) == 0);

// XXX: Requires rustc-1.77
#[cfg(any())]
const _: () = assert!(
    core::mem::align_of::<AppDelegateVar>()
    == core::mem::align_of::<u64>()
);

unsafe impl objc2::encode::RefEncode for AppDelegate {
    const ENCODING_REF: objc2::encode::Encoding = {
        <Self as objc2::ClassType>::Super::ENCODING_REF
    };
}

unsafe impl objc2::Message for AppDelegate {}

unsafe impl icrate::Foundation::NSObjectProtocol for AppDelegate {}
unsafe impl icrate::AppKit::NSApplicationDelegate for AppDelegate {}

unsafe impl objc2::ClassType for AppDelegate {
    type Super = objc2::runtime::NSObject;
    type Mutability = objc2::mutability::MainThreadOnly;

    const NAME: &'static str = "AppDelegate";

    fn class() -> &'static objc2::runtime::AnyClass {
        static REGISTRATION: std::sync::Once = std::sync::Once::new();

        REGISTRATION.call_once(|| {
            let superclass = Self::Super::class();
            let mut builder = objc2::declare::ClassBuilder::new(
                Self::NAME,
                superclass,
            ).unwrap();

            builder.add_ivar::<AppDelegateVar>("var");

            if let Some(v) = {
                <dyn icrate::Foundation::NSObjectProtocol as objc2::ProtocolType>::protocol()
            } {
                builder.add_protocol(v);
            }

            if let Some(v) = {
                <dyn icrate::AppKit::NSApplicationDelegate as objc2::ProtocolType>::protocol()
            } {
                builder.add_protocol(v);
            }

            unsafe {
                builder.add_method(
                    objc2::sel!(init),
                    Self::init as extern "C" fn(_, _) -> _,
                );
                builder.add_method(
                    objc2::sel!(applicationDidFinishLaunching:),
                    Self::application_did_finish_launching as extern "C" fn(_, _, _) -> _,
                );
                builder.add_method(
                    objc2::sel!(applicationWillTerminate:),
                    Self::application_will_terminate as extern "C" fn(_, _, _) -> _,
                );
            }

            let _ = builder.register();
        });

        objc2::runtime::AnyClass::get(Self::NAME).unwrap()
    }

    fn as_super(&self) -> &Self::Super {
        &self.base
    }

    fn as_super_mut(&mut self) -> &mut Self::Super {
        &mut self.base
    }
}

impl AppDelegate {
    pub fn new(
        mtm: icrate::Foundation::MainThreadMarker,
    ) -> objc2::rc::Id<Self> {
        let this = mtm.alloc();
        unsafe { objc2::msg_send_id![this, init] }
    }

    extern "C" fn init(
        &self,
        _sel: objc2::runtime::Sel,
    ) -> Option<&Self> {
        let this: Option<&Self> = unsafe {
            objc2::msg_send![super(self), init]
        };

        this.map(|this| {
            let var = <Self as objc2::ClassType>::class()
                .instance_variable("var").unwrap();
            unsafe {
                var.load_ptr::<AppDelegateVar>(&this.base)
                    .write(Default::default())
            };
            this
        })
    }

    extern "C" fn application_did_finish_launching(
        &self,
        _sel: objc2::runtime::Sel,
        _notification: &icrate::Foundation::NSNotification,
    ) {
        eprintln!("Launching");

        let mtm = icrate::Foundation::MainThreadMarker::new()
            .expect("macOS applications must run on the main-thread");
        let app = icrate::AppKit::NSApplication::sharedApplication(mtm);
        let wnd;

        // Create the main window.
        {
            wnd = unsafe {
                icrate::AppKit::NSWindow::initWithContentRect_styleMask_backing_defer(
                    mtm.alloc(),
                    icrate::Foundation::NSRect {
                        origin: icrate::Foundation::CGPoint { x: 0.0, y: 0.0 },
                        size: icrate::Foundation::CGSize { width: 800.0, height: 600.0 },
                    },
                    icrate::AppKit::NSWindowStyleMaskClosable
                        | icrate::AppKit::NSWindowStyleMaskResizable
                        | icrate::AppKit::NSWindowStyleMaskTitled,
                    icrate::AppKit::NSBackingStoreBuffered,
                    false,
                )
            };

            wnd.setTitle(icrate::Foundation::ns_string!("Osiris Apis Analyzer"));

            let var = <Self as objc2::ClassType>::class()
                .instance_variable("var").unwrap();
            unsafe {
                (*var.load_ptr::<AppDelegateVar>(&self.base)).window = Some(wnd.clone());
            }
        }

        // Create the content view.
        {
            let cv = unsafe {
                icrate::AppKit::NSStackView::initWithFrame(
                    mtm.alloc(),
                    wnd.frame(),
                )
            };
            unsafe {
                cv.setAlignment(
                    icrate::AppKit::NSLayoutAttributeWidth,
                );
                cv.setDistribution(
                    icrate::AppKit::NSStackViewDistributionFill,
                );
                cv.setOrientation(
                    icrate::AppKit::NSUserInterfaceLayoutOrientationVertical,
                );
                cv.setSpacing(0.);
            }

            let log_view = unsafe {
                icrate::AppKit::NSTextView::initWithFrame(
                    mtm.alloc(),
                    icrate::Foundation::NSRect::ZERO,
                )
            };
            unsafe {
                log_view.setEditable(false);
            }

            let log_input = unsafe {
                icrate::AppKit::NSTextField::initWithFrame(
                    mtm.alloc(),
                    icrate::Foundation::NSRect::ZERO,
                )
            };

            unsafe {
                cv.addArrangedSubview(&log_view);
                cv.addArrangedSubview(&log_input);
                wnd.setContentView(Some(&cv));
            }
        }

        // Create the main menu with `Quit` item.
        {
            let menu_main = unsafe {
                icrate::AppKit::NSMenu::initWithTitle(
                    mtm.alloc(),
                    icrate::Foundation::ns_string!("main"),
                )
            };
            let menu_main_app = unsafe {
                icrate::AppKit::NSMenu::initWithTitle(
                    mtm.alloc(),
                    icrate::Foundation::ns_string!("main.app"),
                )
            };
            let item_main_app = unsafe {
                icrate::AppKit::NSMenuItem::initWithTitle_action_keyEquivalent(
                    mtm.alloc(),
                    icrate::Foundation::ns_string!("main.app"),
                    None,
                    icrate::Foundation::ns_string!(""),
                )
            };
            let item_main_app_quit = unsafe {
                icrate::AppKit::NSMenuItem::initWithTitle_action_keyEquivalent(
                    mtm.alloc(),
                    icrate::Foundation::ns_string!("Quit"),
                    Some(objc2::sel!(terminate:)),
                    icrate::Foundation::ns_string!("q"),
                )
            };

            item_main_app.setSubmenu(Some(&menu_main_app));
            menu_main_app.addItem(&item_main_app_quit);
            menu_main.addItem(&item_main_app);
            app.setMainMenu(Some(&menu_main));
        }

        // Finalize window initialization.
        {
            wnd.center();
            wnd.makeKeyAndOrderFront(None);
        }

        eprintln!("Launched");
    }

    extern "C" fn application_will_terminate(
        &self,
        _sel: objc2::runtime::Sel,
        _notification: &icrate::Foundation::NSNotification,
    ) {
        eprintln!("Terminating");
        eprintln!("Terminated");
    }
}

pub struct App {
}

impl App {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn run(&self) -> std::process::ExitCode {
        let mtm = icrate::Foundation::MainThreadMarker::new()
            .expect("macOS applications must run on the main-thread");
        let app: objc2::rc::Id<_> = {
            icrate::AppKit::NSApplication::sharedApplication(mtm)
        };

        app.setActivationPolicy(icrate::AppKit::NSApplicationActivationPolicyRegular);

        let app_delegate = AppDelegate::new(mtm);
        app.setDelegate(Some(
            objc2::runtime::ProtocolObject::from_ref(&*app_delegate),
        ));

        unsafe {
            app.run()
        };

        0.into()
    }
}
