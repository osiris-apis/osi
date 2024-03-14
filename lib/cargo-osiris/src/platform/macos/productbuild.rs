//! # macOS Product Builder
//!
//! The `productbuild` utility is a standard macOS helper to build packages.
//! This module provides structured access to its features.

use crate::op;

/// Combined arguments to a build-query
pub struct BuildQuery<'ctx, ComponentList> {
    /// Path to the input files to merge into the plist file
    pub components: ComponentList,
    /// Signing identity to use
    pub identity: Option<&'ctx str>,
    /// Path to the output file
    pub output_file: &'ctx std::path::Path,
}

impl<'ctx, ComponentList> BuildQuery<'ctx, ComponentList>
where
    ComponentList: Clone + Iterator<Item = &'ctx (&'ctx std::path::Path, &'ctx std::path::Path)>,
{
    /// Execute a build-query via the `productbuild` utility of macOS.
    pub fn run(&self) -> Result<(), op::ErrorProcess> {
        let mut cmd = std::process::Command::new("xcrun");

        cmd.arg("productbuild");

        for (from, to) in self.components.clone() {
            cmd.arg("--component");
            cmd.arg(from);
            cmd.arg(to);
        }

        if let Some(v) = self.identity {
            cmd.arg("--sign");
            cmd.arg(v);
        }

        cmd.arg("--");
        cmd.arg(self.output_file);

        cmd.stderr(std::process::Stdio::inherit());
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::inherit());

        let status = cmd.status()
            .map_err(|io| op::ErrorProcess::Exec { name: "productbuild".into(), io })?;
        if !status.success() {
            return Err(op::ErrorProcess::Exit { name: "productbuild".into(), code: status });
        }

        Ok(())
    }
}
