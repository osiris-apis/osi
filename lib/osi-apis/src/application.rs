//! # Application Context
//!
//! The application context represents the local application and provides
//! access to system APIs that manage or track the application. It is used
//! by several modules to interact with the system representation of the
//! local application.

/// ## Application Setup
///
/// The setup structure contains all the parameters required to initialize
/// the application.
pub struct Setup<'ctx> {
    pub(crate) native: crate::native::application::Setup<'ctx>,
}

/// ## Application Context
///
/// The context of the local application, providing access to system APIs
/// regarding the state and lifetime of the application.
pub struct Context {
    pub(crate) native: crate::native::application::Context,
}

impl<'ctx> From<crate::native::application::Setup<'ctx>> for Setup<'ctx> {
    fn from(v: crate::native::application::Setup<'ctx>) -> Self {
        Self {
            native: v,
        }
    }
}

impl From<crate::native::application::Context> for Context {
    fn from(v: crate::native::application::Context) -> Self {
        Self {
            native: v,
        }
    }
}

impl<'ctx> Setup<'ctx> {
    /// ## Initialize the Application
    ///
    /// Perform all application initialization and yield the application
    /// context ready to be used.
    pub fn initialize(
        &self,
    ) -> Result<Context, Box<dyn std::error::Error>> {
        self.native.initialize().map(|v| v.into())
    }
}

impl Context {
    /// ## Yield Native Context
    ///
    /// Return a reference to the underlying native context.
    pub fn native(&self) -> &crate::native::application::Context {
        &self.native
    }
}
