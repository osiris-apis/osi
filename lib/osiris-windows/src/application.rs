//! # Application Context
//!
//! The application context represents the local application and provides
//! access to system APIs that manage or track the application. It is used
//! by several modules to interact with the system representation of the
//! local application.

/// ## Application Context
///
/// The context of the local application, providing access to system APIs
/// regarding the state and lifetime of the application.
pub struct Application {
    pub(crate) id: String,
}

impl Application {
    /// ## Create with an Application ID
    ///
    /// Create a new application context with the given application ID. Note
    /// that application IDs have a well-defined meaning on Windows and must
    /// be properly acquired through external means.
    pub fn with_id(id: &str) -> Self {
        Self {
            id: id.into(),
        }
    }

    /// ## Create Application Context
    ///
    /// Create a new application context.
    pub fn new() -> Self {
        // XXX: Application IDs are required for many operations. Provide a
        //      dummy to allow tests to go through.
        //      This should be dropped once the application model is fully
        //      figured out.
        Self::with_id(
            "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe",
        )
    }

    /// ## Perform Application Setup
    ///
    /// Perform platform dependent setup operations for the application
    /// context. This includes operations like establishing COM connections
    /// or populating caches.
    pub fn setup(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
