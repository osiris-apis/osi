//! # Application Context
//!
//! The application context represents the local application and provides
//! access to system APIs that manage or track the application. It is used
//! by several modules to interact with the system representation of the
//! local application.

use gio;

/// ## Application Setup
///
/// The setup structure contains all the parameters required to initialize
/// the application.
#[derive(Default)]
pub struct Setup<'ctx> {
    /// Application identifier for the running application, or `None` to
    /// run without identifier. Note that application IDs follow strict rules
    /// (usually ASCII-only reverse domain names; see `gio/Application`).
    pub id: Option<&'ctx str>,
}

/// ## Application Context
///
/// The context of the local application, providing access to system APIs
/// regarding the state and lifetime of the application.
pub struct Context {
    pub(crate) gio: gio::Application,
}

impl<'ctx> Setup<'ctx> {
    /// ## Initialize the Application
    ///
    /// Perform all application initialization and yield the application
    /// context ready to be used.
    pub fn initialize(
        &self,
    ) -> Result<Context, Box<dyn std::error::Error>> {
        let v_gio = gio::Application::new(
            match self.id {
                // XXX: The application ID is not used for `NON_UNIQUE`
                //      applications, yet gio has some weird behavior if
                //      none is set. We use a dummy value for now, but
                //      this should be solved properly.
                None => Some("foo.osiris.unknown"),
                Some(ref v) => Some(v),
            },
            gio::ApplicationFlags::NON_UNIQUE,
        );

        <_ as gio::prelude::ApplicationExt>::register(
            &v_gio,
            None::<&gio::Cancellable>,
        ).map_err(|v| Box::new(v))?;

        Ok(Context {
            gio: v_gio,
        })
    }
}
