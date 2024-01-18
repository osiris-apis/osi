//! # Osiris APIs for Windows
//!
//! This is an implementation of the Osiris APIs for the Windows platform. It
//! uses Windows 8.1 as baseline, but can make use of newer Windows features
//! if available.

#![cfg(any(all(doc, not(doctest)), target_os = "windows"))]

pub mod application;
pub mod notification;
