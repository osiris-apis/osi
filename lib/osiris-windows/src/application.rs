//! # Application Context
//!
//! The application context represents the local application and provides
//! access to system APIs that manage or track the application. It is used
//! by several modules to interact with the system representation of the
//! local application.

use windows;

/// ## Application Setup
///
/// The setup structure contains all the parameters required to initialize
/// the application.
pub struct Setup<'ctx> {
    /// Application User Model ID to use, or `None` to inherit it.
    pub aumid: Option<&'ctx str>,
}

/// ## Application Context
///
/// The context of the local application, providing access to system APIs
/// regarding the state and lifetime of the application.
pub struct Context {
    aumid: Option<windows::core::HSTRING>,
}

impl<'ctx> Setup<'ctx> {
    /// ## Initialize the Application
    ///
    /// Perform all application initialization and yield the application
    /// context ready to be used.
    pub fn initialize(
        &self,
    ) -> Result<Context, Box<dyn std::error::Error>> {
        // If the caller supplied a AUMID, ensure that we set it on the current
        // process, so every created entity can inherit it. If none was
        // provided, query the current value (if any) and cache it.
        let v_aumid = if let Some(id) = self.aumid {
            let hstr = windows::core::HSTRING::from(id);

            unsafe {
                windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(
                    &hstr,
                )?;
            }

            Some(hstr)
        } else {
            unsafe {
                windows::Win32::UI::Shell::GetCurrentProcessExplicitAppUserModelID()
                    .and_then(|v| v.to_hstring())
                    .ok()
            }
        };

        Ok(Context {
            aumid: v_aumid,
        })
    }
}

impl Context {
    /// ## Yield AUMID
    ///
    /// Yield the AUMID of the application, if any.
    ///
    /// The Application User Model ID is used to group runtime entities of
    /// an application (e.g., windows and processes). An application can use
    /// multiple ones in a single process to behave as multiple logical
    /// applications. This field yields the ID that was configured for the
    /// application root process. If no such ID was configured, `None` is
    /// returned.
    pub fn aumid(&self) -> Option<windows::core::HSTRING> {
        self.aumid.clone()
    }

    /// ## Yield AUMID or a Fallback Value
    ///
    /// Yield the AUMID of the application, but use a fallback value if none
    /// was set or inherited.
    ///
    /// The fallback value represents an application (usually PowerShell) that
    /// is always available on the system, and thus can be used for
    /// development purposes.
    ///
    /// Use this function with care! Only use it if a fallback value is
    /// actually desired.
    pub fn aumid_or_fallback(&self) -> windows::core::HSTRING {
        self.aumid()
            .unwrap_or_else(
                || windows::core::HSTRING::from(
                    "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe",
                ),
            )
    }

    /// ## Claim the Context
    ///
    /// A no-op that is used to mark variables as used and prevent them from
    /// being dropped by refactors.
    ///
    /// Application contexts are often implicit by guaranteeing that some
    /// information of the calling process has been pre-populated. Use this
    /// function to document that you rely on this behavior.
    pub fn claim(&self) {
        // no-op
    }
}
