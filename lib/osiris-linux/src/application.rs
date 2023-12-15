//! # Application Context
//!
//! The application context represents the local application and provides
//! access to system APIs that manage or track the application. It is used
//! by several modules to interact with the system representation of the
//! local application.

use gio;

/// ## Application Context
///
/// The context of the local application, providing access to system APIs
/// regarding the state and lifetime of the application.
pub struct Application {
    pub(crate) gio: gio::Application,
}

impl Application {
    /// ## Create with an Application ID
    ///
    /// Create a new application context with the given application ID. Note
    /// that application IDs follow strict rules (usually ASCII-only reverse
    /// domain names; see `gio/Application`).
    pub fn with_id(id: &str) -> Self {
        Self {
            gio: gio::Application::new(
                Some(id),
                gio::ApplicationFlags::NON_UNIQUE,
            ),
        }
    }

    /// ## Create Application Context
    ///
    /// Create a new application context.
    pub fn new() -> Self {
        // XXX: The application ID is not used for `NON_UNIQUE` applications,
        //      yet gio is buggy if no ID is set. Hence, we provide a dummy
        //      value. See glib-issue #3203 for our blocker.
        Self::with_id("foo.osiris.unknown")
    }

    /// ## Perform Application Setup
    ///
    /// Perform platform dependent setup operations for the application
    /// context. This includes operations like connecting to the system bus
    /// or populating caches.
    pub fn setup(&self) -> Result<(), Box<dyn std::error::Error>> {
        <_ as gio::prelude::ApplicationExt>::register(
            &self.gio,
            None::<&gio::Cancellable>,
        ).map_err(|v| Box::new(v))?;

        Ok(())
    }
}
