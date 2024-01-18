//! # Osiris APIs for macOS
//!
//! This is an implementation of the Osiris APIs for macOS. It uses the C and
//! Objective-C APIs of the macOS platform to communicate with the platform.

#![cfg(any(all(doc, not(doctest)), target_os = "macos"))]

pub mod application;
