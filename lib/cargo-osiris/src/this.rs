//! Process Context
//!
//! This module provides a custom process context with access to all global
//! entities. Any module that needs access to one of these contexts must
//! thus be passed the process context.

/// Process context with exclusive access to global entities, parameters and
/// communication channels.
pub struct This {
    // Standard I/O
    display: std::io::Stderr,
    input: std::io::Stdin,
    output: std::io::Stdout,

    // Task properties
    workdir: std::path::PathBuf,
}

impl This {
    fn with(
        display: std::io::Stderr,
        input: std::io::Stdin,
        output: std::io::Stdout,
        workdir: std::path::PathBuf,
    ) -> Self {
        Self {
            display: display,
            input: input,
            output: output,
            workdir: workdir,
        }
    }

    /// Create a new process context from ambient capabilities.
    ///
    /// This will query ambient capabilities of the process and create the
    /// process context from it. All information is copied at the time of this
    /// call and thus will represent the ambient capabilities of the process
    /// at this time. Later changes to the ambient capabilities of the process
    /// will (intentionally) not reflect into the context.
    ///
    /// This function will assume that ambient process capabilities are
    /// accessible. It will panic if not.
    pub fn from_ambient() -> Self {
        let v_display = std::io::stderr();
        let v_input = std::io::stdin();
        let v_output = std::io::stdout();
        let v_workdir = std::env::current_dir().expect("Current working directory must be set");

        Self::with(
            v_display,
            v_input,
            v_output,
            v_workdir,
        )
    }

    /// Yield access to the display abstraction.
    pub fn display(&mut self) -> &mut std::io::Stderr {
        &mut self.display
    }

    /// Yield access to the process-input abstraction.
    pub fn input(&mut self) -> &mut std::io::Stdin {
        &mut self.input
    }

    /// Yield access to the process-output abstraction.
    pub fn output(&mut self) -> &mut std::io::Stdout {
        &mut self.output
    }

    /// Yield access to the working directory.
    pub fn workdir(&self) -> &std::path::Path {
        &self.workdir
    }
}
