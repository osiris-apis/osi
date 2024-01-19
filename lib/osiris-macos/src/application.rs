//! # Application Context
//!
//! The application context represents the local application and provides
//! access to system APIs that manage or track the application. It is used
//! by several modules to interact with the system representation of the
//! local application.

use icrate;
use objc2;

/// ## Application Setup
///
/// The setup structure contains all the parameters required to initialize
/// the application.
#[derive(Default)]
pub struct Setup<'ctx> {
    /// Application identifier for the running application, or `None` to
    /// run without identifier.
    pub id: Option<&'ctx str>,
}

/// ## Application Context
///
/// The context of the local application, providing access to system APIs
/// regarding the state and lifetime of the application.
pub struct Context {
    pub(crate) app: objc2::rc::Id<icrate::AppKit::NSApplication>,
}

impl<'ctx> Setup<'ctx> {
    /// ## Initialize the Application
    ///
    /// Perform all application initialization and yield the application
    /// context ready to be used.
    pub fn initialize(
        &self,
    ) -> Result<Context, Box<dyn std::error::Error>> {
        let mtm = icrate::Foundation::MainThreadMarker::new()
            .ok_or::<Box<dyn std::error::Error>>(
                "error: cannot create application on non-main thread".into(),
            )?;

        Ok(Context {
            app: icrate::AppKit::NSApplication::sharedApplication(mtm),
        })
    }
}

impl Context {
    /// ## Run Application Main-Loop
    ///
    /// Enter the main-loop of the application context and dispatch messages
    /// until the application is terminated.
    pub fn run(
        &self,
    ) {
        unsafe {
            self.app.run();
        }
    }
}
