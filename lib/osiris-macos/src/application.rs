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
}

impl<'ctx> Setup<'ctx> {
    /// ## Initialize the Application
    ///
    /// Perform all application initialization and yield the application
    /// context ready to be used.
    pub fn initialize(
        &self,
    ) -> Result<Context, Box<dyn std::error::Error>> {
        Ok(Context {
        })
    }
}
