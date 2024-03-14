//! # macOS Code Signing
//!
//! This module provides access to the macOS Code Signing machinery. This is
//! used by macOS to sign and verify the integrity and origin of code and its
//! resources.

use crate::op;

/// Enumeration of options that can be embedded during signing and control how
/// the signature will be used.
///
/// The relevant authoritative source is `SecCodeSignatureFlags` in the macOS
/// SDK, defining all possible public options.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum SignOption {
    Host,
    AdHoc,
    ForceHard,
    ForceKill,
    ForceExpiration,
    Restrict,
    Enforcement,
    LibraryValidation,
    Runtime,
    LinkerSigned,
}

/// Combined arguments to a signing query via `codesign`.
pub struct SignQuery<
    'ctx,
    OptionList,
    PathList,
> {
    /// Path to entitlement file
    pub entitlements: Option<&'ctx std::path::Path>,
    /// Whether to override previous signatures
    pub force: bool,
    /// Signing identity to use
    pub identity: &'ctx str,
    /// Signing options to apply
    pub options: Option<OptionList>,
    /// Object paths to sign
    pub paths: PathList,
    /// Internal requirements to embed
    pub requirements: Option<&'ctx str>,
    /// Whether to contact timestamp authority servers
    pub timestamp: Option<bool>,
}

impl SignOption {
    /// Return canonical integer representation of the option.
    ///
    /// The relevant authoritative sources is `SecCodeSignatureFlags` as
    /// exposed by `Security.framework`.
    pub fn as_u32(&self) -> u32 {
        match self {
            SignOption::Host                => 0x0000_0001,
            SignOption::AdHoc               => 0x0000_0002,
            SignOption::ForceHard           => 0x0000_0100,
            SignOption::ForceKill           => 0x0000_0200,
            SignOption::ForceExpiration     => 0x0000_0400,
            SignOption::Restrict            => 0x0000_0800,
            SignOption::Enforcement         => 0x0000_1000,
            SignOption::LibraryValidation   => 0x0000_2000,
            SignOption::Runtime             => 0x0001_0000,
            SignOption::LinkerSigned        => 0x0002_0000,
        }
    }

    /// Return canonical string representation of the option.
    ///
    /// The relevant authoritative sources is `kSecCodeDirectoryFlagTable` as
    /// exposed by `Security.framework`.
    pub fn as_str(&self) -> &str {
        match self {
            SignOption::Host => "host",
            SignOption::AdHoc => "adhoc",
            SignOption::ForceHard => "hard",
            SignOption::ForceKill => "kill",
            SignOption::ForceExpiration => "expires",
            SignOption::Restrict => "restrict",
            SignOption::Enforcement => "enforcement",
            SignOption::LibraryValidation => "library-validation",
            SignOption::Runtime => "runtime",
            SignOption::LinkerSigned => "linker-signed",
        }
    }
}

impl<
    'ctx,
    OptionList,
    PathList,
> SignQuery<'ctx, OptionList, PathList>
where
    OptionList: Clone + Iterator<Item = SignOption>,
    PathList: Clone + Iterator,
    <PathList as Iterator>::Item: AsRef<std::path::Path>,
{
    /// Execute a signing query via the `codesign` utility of the macOS SDK.
    /// This will run `codesign` with the `--sign` flag to perform a
    /// signing operation.
    pub fn run(&self) -> Result<(), op::ErrorProcess> {
        let mut cmd = std::process::Command::new("codesign");

        // Select signing operation and specify the signing identity.
        cmd.arg("--sign");
        cmd.arg(self.identity);

        // Generate the newer DER format, in case we run with a `codesign`
        // version that does not do that by default.
        cmd.arg("--generate-entitlement-der");

        // Strip extended attributes, as they are not covered by signatures.
        cmd.arg("--strip-disallowed-xattrs");

        // Embed entitlements in the signature.
        if let Some(v) = self.entitlements {
            cmd.arg("--entitlements");
            cmd.arg(v);
        }

        // Override previous signatures, if desired.
        if self.force {
            cmd.arg("--force");
        }

        // Append stringified version of all options.
        if let Some(ref options) = self.options {
            let mut acc = String::new();
            let mut first = true;

            for o in options.clone() {
                if first {
                    first = false;
                } else {
                    acc += ",";
                }
                acc += o.as_str();
            }

            if !first {
                cmd.arg("--options");
                cmd.arg(acc);
            }
        }

        // Add internal requirements, if requested.
        if let Some(v) = self.requirements {
            cmd.arg("--requirements");
            cmd.arg(v);
        }

        // Request use of timestamp authority servers, if desired.
        if let Some(v) = self.timestamp {
            if v {
                cmd.arg("--timestamp");
            } else {
                cmd.arg("--timestamp=none");
            }
        }

        // Finalize the flags and then append all paths verbatim.
        cmd.arg("--");
        for v in self.paths.clone() {
            cmd.arg(v.as_ref());
        }

        cmd.stderr(std::process::Stdio::inherit());

        let output = cmd.output()
            .map_err(|io| op::ErrorProcess::Exec { name: "codesign".into(), io })?;
        if !output.status.success() {
            return Err(op::ErrorProcess::Exit { name: "codesign".into(), code: output.status });
        }

        Ok(())
    }
}
