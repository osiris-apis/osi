//! Platform Layer: macOS
//!
//! Implement the application and UI via Cocoa, using the classic NSWindow UI
//! handling.
//!
//! The UI uses a simple text-view to show all output, and an input-entry
//! to accept commands from the user.

use icrate;
use objc2;

#[repr(C)]
pub struct AppDelegate {
    // Base class must be at offset 0.
    base: objc2::runtime::NSObject,
}

// XXX: Requires rustc-1.77
#[cfg(any())]
const _: () = assert!(core::mem::offset_of!(AppDelegate, base) == 0);

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

    extern "C" fn application_did_finish_launching(
        &self,
        _sel: objc2::runtime::Sel,
        _notification: &icrate::Foundation::NSNotification,
    ) {
        eprintln!("Launching...");
    }

    extern "C" fn application_will_terminate(
        &self,
        _sel: objc2::runtime::Sel,
        _notification: &icrate::Foundation::NSNotification,
    ) {
        eprintln!("Terminating...");
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
