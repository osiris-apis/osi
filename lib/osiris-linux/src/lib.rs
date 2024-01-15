//! # Osiris APIs for Linux
//!
//! This is an implementation of the Osiris APIs for the Linux platform. While
//! the Linux platform is quite heterogeneous, a lot of common APIs are
//! supported across the different platforms. The Freedesktop Project hosts
//! a standardization effort and provides many specifications for
//! interoperability.
//!
//! This implementation currently relies on the GTK/glib libraries as client
//! implementation of these specifications. However, this does not tie it to
//! GNOME systems, but can be used on any compatible platform. Furthermore, the
//! public API does not expose any of the GTK/glib types, and thus the project
//! can switch to a custom implementation of the specifications in the future.

#![cfg(any(doc, target_os = "linux"))]

pub mod application;
pub mod notification;
