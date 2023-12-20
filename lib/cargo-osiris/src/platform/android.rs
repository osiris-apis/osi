//! # Android Platform Support
//!
//! This module provides build-system support for the Android platform. It
//! supports direct builds via the Android SDK, or following the official
//! Gradle build system.

use crate::{cargo, config, lib, op};
use std::collections::BTreeMap;

mod apk;
mod dex;
mod flatres;
mod java;
mod kotlin;
mod sdk;

/// ## Android Platform Build Errors
///
/// This is an extension of `op::BuildError` with all errors specific to
/// building on the Android platform.
#[derive(Debug)]
pub enum BuildError {
    /// Path contains characters that are not supported by the required tools.
    /// This very likely means the path contains colons or semicolons.
    UnsupportedPath(std::path::PathBuf),
    /// Host platform not supported by the Android SDK.
    UnsupportedHost,
    /// Unsupported target ABI for the Android platform.
    UnsupportedAbi(String),
    /// No Android SDK available, `ANDROID_HOME` is not set.
    NoAndroidHome,
    /// No Android SDK available at the selected location.
    NoSdk(std::path::PathBuf),
    /// Invalid Android SDK at the selected location.
    InvalidSdk(std::path::PathBuf),
    /// No Android Java SDK available at the selected location.
    NoJdk(std::path::PathBuf),
    /// Invalid Android Java SDK at the selected location.
    InvalidJdk(std::path::PathBuf),
    /// No Android Kotlin SDK available at the selected location.
    NoKdk(std::path::PathBuf),
    /// Invalid Android Kotlin SDK at the selected location.
    InvalidKdk(std::path::PathBuf),
    /// No NDK available in the selected Android SDK.
    NoNdk,
    /// Invalid NDK with the selected version in the Android SDK.
    InvalidNdk(std::ffi::OsString),
    /// No Build Tools available in the selected Android SDK.
    NoBuildTools,
    /// Invalid Build Tools with the selected version in the Android SDK.
    InvalidBuildTools(std::ffi::OsString),
    /// No platform for the selected API-level available in the selected
    /// Android SDK.
    NoPlatform(u32),
    /// Invalid platform with the selected API-level in the selected
    /// Android SDK.
    InvalidPlatform(u32),
    /// Execution of the Android resource compiler could not commence.
    FlatresExec(std::io::Error),
    /// Android resource compiler failed executing.
    FlatresExit(std::process::ExitStatus),
    /// Execution of the Java compiler could not commence.
    JavacExec(std::io::Error),
    /// Java compiler failed executing.
    JavacExit(std::process::ExitStatus),
    /// Execution of the Kotlin compiler could not commence.
    KotlincExec(std::io::Error),
    /// Kotlin compiler failed executing.
    KotlincExit(std::process::ExitStatus),
    /// Execution of the DEX compiler could not commence.
    DexExec(std::io::Error),
    /// DEX compiler failed executing.
    DexExit(std::process::ExitStatus),
    /// Execution of the Android APK linker could not commence.
    AaptExec(std::io::Error),
    /// Android APK linker failed executing.
    AaptExit(std::process::ExitStatus),
}

struct Build<'ctx> {
    // Configuration
    pub android: &'ctx config::ConfigPlatformAndroid,
    pub build_dir: &'ctx std::path::Path,
    pub config: &'ctx config::Config,
    pub metadata: &'ctx cargo::Metadata,
    pub platform: &'ctx config::ConfigPlatform,

    // Build directories
    pub apk_dir: std::path::PathBuf,
    pub artifact_dir: std::path::PathBuf,
    pub class_dir: std::path::PathBuf,
    pub dex_dir: std::path::PathBuf,
    pub java_dir: std::path::PathBuf,
    pub resource_dir: std::path::PathBuf,

    // Artifact files
    pub apk_aligned_file: std::path::PathBuf,
    pub apk_base_file: std::path::PathBuf,
    pub apk_linked_file: std::path::PathBuf,
    pub apk_signed_file: std::path::PathBuf,
    pub classes_dex_file: std::path::PathBuf,
    pub debug_keystore_file: std::path::PathBuf,
    pub manifest_file: std::path::PathBuf,
}

struct Direct<'ctx> {
    // Build context
    pub build: &'ctx Build<'ctx>,

    // Tools and resources
    pub build_tools: sdk::BuildTools,
    pub jdk: sdk::Jdk,
    pub kdk: sdk::Kdk,
    pub ndk: sdk::Ndk,
    pub platform: std::path::PathBuf,
    pub platform_jar: std::path::PathBuf,
    pub sdk: sdk::Sdk,
}

// ## Debug Keystore
//
// This is the built-in keystore with a debugging key. It has been created
// with the standard Java tool `keytool`, with the following arguments:
//
// ```sh
// keytool \
//     -genkey \
//     -v \
//     -alias android \
//     -keyalg ed25519 \
//     -keypass android \
//     -keystore debug.keystore \
//     -storepass android \
//     -validity 46720
// ```
//
// Android Studio uses a similar `debug.keystore` file to sign applications
// during development. We store it in the target directory for everyone to
// use.
const DEBUG_KEYSTORE: [u8; 1110] = [
    b'\x30', b'\x82', b'\x04', b'\x52', b'\x02', b'\x01', b'\x03', b'\x30',
    b'\x82', b'\x03', b'\xfc', b'\x06', b'\x09', b'\x2a', b'\x86', b'\x48',
    b'\x86', b'\xf7', b'\x0d', b'\x01', b'\x07', b'\x01', b'\xa0', b'\x82',
    b'\x03', b'\xed', b'\x04', b'\x82', b'\x03', b'\xe9', b'\x30', b'\x82',
    b'\x03', b'\xe5', b'\x30', b'\x82', b'\x01', b'\x1c', b'\x06', b'\x09',
    b'\x2a', b'\x86', b'\x48', b'\x86', b'\xf7', b'\x0d', b'\x01', b'\x07',
    b'\x01', b'\xa0', b'\x82', b'\x01', b'\x0d', b'\x04', b'\x82', b'\x01',
    b'\x09', b'\x30', b'\x82', b'\x01', b'\x05', b'\x30', b'\x82', b'\x01',
    b'\x01', b'\x06', b'\x0b', b'\x2a', b'\x86', b'\x48', b'\x86', b'\xf7',
    b'\x0d', b'\x01', b'\x0c', b'\x0a', b'\x01', b'\x02', b'\xa0', b'\x81',
    b'\xad', b'\x30', b'\x81', b'\xaa', b'\x30', b'\x66', b'\x06', b'\x09',
    b'\x2a', b'\x86', b'\x48', b'\x86', b'\xf7', b'\x0d', b'\x01', b'\x05',
    b'\x0d', b'\x30', b'\x59', b'\x30', b'\x38', b'\x06', b'\x09', b'\x2a',
    b'\x86', b'\x48', b'\x86', b'\xf7', b'\x0d', b'\x01', b'\x05', b'\x0c',
    b'\x30', b'\x2b', b'\x04', b'\x14', b'\xb4', b'\xdf', b'\x30', b'\x45',
    b'\xb5', b'\x40', b'\x0c', b'\xf6', b'\x48', b'\x3c', b'\x39', b'\xa6',
    b'\x67', b'\x7e', b'\x13', b'\x35', b'\x2b', b'\x91', b'\x62', b'\x76',
    b'\x02', b'\x02', b'\x27', b'\x10', b'\x02', b'\x01', b'\x20', b'\x30',
    b'\x0c', b'\x06', b'\x08', b'\x2a', b'\x86', b'\x48', b'\x86', b'\xf7',
    b'\x0d', b'\x02', b'\x09', b'\x05', b'\x00', b'\x30', b'\x1d', b'\x06',
    b'\x09', b'\x60', b'\x86', b'\x48', b'\x01', b'\x65', b'\x03', b'\x04',
    b'\x01', b'\x2a', b'\x04', b'\x10', b'\x11', b'\x3b', b'\x66', b'\xef',
    b'\x53', b'\x84', b'\x2e', b'\xa9', b'\x6c', b'\x89', b'\xae', b'\x2a',
    b'\xed', b'\x91', b'\x4a', b'\xf4', b'\x04', b'\x40', b'\xb1', b'\xab',
    b'\x06', b'\xbb', b'\xfc', b'\xd5', b'\xc0', b'\x6c', b'\xea', b'\x04',
    b'\xc6', b'\xe7', b'\x70', b'\x90', b'\xc1', b'\x5c', b'\x8e', b'\xc7',
    b'\x63', b'\x45', b'\xef', b'\x31', b'\xc1', b'\x73', b'\x0e', b'\x59',
    b'\xde', b'\xb2', b'\xb9', b'\xe8', b'\x59', b'\xe2', b'\xda', b'\xc3',
    b'\x68', b'\x3e', b'\xb5', b'\xb5', b'\x7e', b'\x96', b'\x9c', b'\x33',
    b'\xf1', b'\x58', b'\xe1', b'\x57', b'\x52', b'\x31', b'\x6c', b'\xd2',
    b'\x1e', b'\x72', b'\xfa', b'\x1d', b'\xca', b'\xb4', b'\x2b', b'\x58',
    b'\x0e', b'\x3b', b'\xb0', b'\xee', b'\xb3', b'\xce', b'\x31', b'\x42',
    b'\x30', b'\x1d', b'\x06', b'\x09', b'\x2a', b'\x86', b'\x48', b'\x86',
    b'\xf7', b'\x0d', b'\x01', b'\x09', b'\x14', b'\x31', b'\x10', b'\x1e',
    b'\x0e', b'\x00', b'\x61', b'\x00', b'\x6e', b'\x00', b'\x64', b'\x00',
    b'\x72', b'\x00', b'\x6f', b'\x00', b'\x69', b'\x00', b'\x64', b'\x30',
    b'\x21', b'\x06', b'\x09', b'\x2a', b'\x86', b'\x48', b'\x86', b'\xf7',
    b'\x0d', b'\x01', b'\x09', b'\x15', b'\x31', b'\x14', b'\x04', b'\x12',
    b'\x54', b'\x69', b'\x6d', b'\x65', b'\x20', b'\x31', b'\x37', b'\x30',
    b'\x33', b'\x30', b'\x37', b'\x35', b'\x32', b'\x30', b'\x35', b'\x36',
    b'\x38', b'\x38', b'\x30', b'\x82', b'\x02', b'\xc1', b'\x06', b'\x09',
    b'\x2a', b'\x86', b'\x48', b'\x86', b'\xf7', b'\x0d', b'\x01', b'\x07',
    b'\x06', b'\xa0', b'\x82', b'\x02', b'\xb2', b'\x30', b'\x82', b'\x02',
    b'\xae', b'\x02', b'\x01', b'\x00', b'\x30', b'\x82', b'\x02', b'\xa7',
    b'\x06', b'\x09', b'\x2a', b'\x86', b'\x48', b'\x86', b'\xf7', b'\x0d',
    b'\x01', b'\x07', b'\x01', b'\x30', b'\x66', b'\x06', b'\x09', b'\x2a',
    b'\x86', b'\x48', b'\x86', b'\xf7', b'\x0d', b'\x01', b'\x05', b'\x0d',
    b'\x30', b'\x59', b'\x30', b'\x38', b'\x06', b'\x09', b'\x2a', b'\x86',
    b'\x48', b'\x86', b'\xf7', b'\x0d', b'\x01', b'\x05', b'\x0c', b'\x30',
    b'\x2b', b'\x04', b'\x14', b'\x82', b'\x6c', b'\x0b', b'\x31', b'\xf6',
    b'\x94', b'\x75', b'\x85', b'\x92', b'\xf6', b'\x54', b'\x69', b'\xdb',
    b'\xa9', b'\xf3', b'\xd6', b'\x93', b'\xd9', b'\x09', b'\x63', b'\x02',
    b'\x02', b'\x27', b'\x10', b'\x02', b'\x01', b'\x20', b'\x30', b'\x0c',
    b'\x06', b'\x08', b'\x2a', b'\x86', b'\x48', b'\x86', b'\xf7', b'\x0d',
    b'\x02', b'\x09', b'\x05', b'\x00', b'\x30', b'\x1d', b'\x06', b'\x09',
    b'\x60', b'\x86', b'\x48', b'\x01', b'\x65', b'\x03', b'\x04', b'\x01',
    b'\x2a', b'\x04', b'\x10', b'\x49', b'\xfa', b'\x58', b'\xba', b'\x2c',
    b'\x5c', b'\xcb', b'\x1c', b'\x19', b'\x42', b'\x03', b'\xf0', b'\x32',
    b'\xf7', b'\xc2', b'\x31', b'\x80', b'\x82', b'\x02', b'\x30', b'\x26',
    b'\x3c', b'\x36', b'\xdf', b'\xef', b'\x59', b'\xad', b'\x9d', b'\x85',
    b'\x57', b'\xbf', b'\x9b', b'\xa9', b'\x5a', b'\xfa', b'\xfe', b'\xb0',
    b'\x27', b'\x6a', b'\x59', b'\x8d', b'\x2b', b'\xa3', b'\xe7', b'\xe3',
    b'\x2e', b'\xea', b'\x6e', b'\x37', b'\x01', b'\xaa', b'\x00', b'\x6d',
    b'\x7b', b'\x15', b'\x4a', b'\x39', b'\xf9', b'\x08', b'\xca', b'\xfd',
    b'\x51', b'\x8d', b'\x60', b'\x13', b'\x4c', b'\x04', b'\x9d', b'\x8d',
    b'\x26', b'\x18', b'\xa1', b'\xff', b'\xbe', b'\x21', b'\x95', b'\xda',
    b'\x6b', b'\x0d', b'\x35', b'\x47', b'\x4c', b'\x1d', b'\xb4', b'\x2c',
    b'\x0b', b'\x55', b'\x5d', b'\xd5', b'\xbe', b'\x97', b'\xea', b'\x7a',
    b'\x8f', b'\xac', b'\xc3', b'\x2b', b'\x47', b'\x2b', b'\x00', b'\xe3',
    b'\x13', b'\xf0', b'\xe4', b'\xb4', b'\x99', b'\x30', b'\x5b', b'\xbe',
    b'\xdc', b'\x33', b'\x9d', b'\x8a', b'\xb0', b'\xa0', b'\x31', b'\xab',
    b'\xa7', b'\xd8', b'\x19', b'\xef', b'\x32', b'\xb7', b'\xe1', b'\xac',
    b'\x12', b'\x93', b'\xe4', b'\x20', b'\xa2', b'\xb6', b'\x09', b'\xf6',
    b'\xe9', b'\x97', b'\x87', b'\xa8', b'\xbc', b'\xc2', b'\x1a', b'\xb2',
    b'\xc6', b'\x82', b'\x23', b'\x61', b'\xb9', b'\xd5', b'\xc3', b'\xe2',
    b'\xd3', b'\x0c', b'\x82', b'\xf0', b'\x4c', b'\x19', b'\x8c', b'\xaf',
    b'\x9b', b'\xfb', b'\x06', b'\x3e', b'\x6b', b'\x76', b'\x23', b'\x5b',
    b'\x71', b'\xd6', b'\xc2', b'\x0f', b'\x9f', b'\xbc', b'\x19', b'\x68',
    b'\x21', b'\x77', b'\xfe', b'\xcb', b'\x57', b'\xe3', b'\x78', b'\x6b',
    b'\x14', b'\x62', b'\x48', b'\x6e', b'\x88', b'\x78', b'\x99', b'\x1c',
    b'\xdd', b'\xa6', b'\x71', b'\xe2', b'\x2d', b'\xde', b'\xc6', b'\x73',
    b'\x1b', b'\x52', b'\x3c', b'\x7e', b'\x83', b'\xea', b'\x28', b'\x0e',
    b'\xa1', b'\x58', b'\xcc', b'\xcc', b'\xd8', b'\x8e', b'\xc2', b'\xc3',
    b'\x5e', b'\x64', b'\xfe', b'\x34', b'\x8d', b'\x98', b'\x5b', b'\x29',
    b'\x91', b'\x94', b'\x2d', b'\x61', b'\xc1', b'\x8e', b'\xf2', b'\x6c',
    b'\xe4', b'\x1d', b'\x4c', b'\x5a', b'\xc4', b'\xf2', b'\x0a', b'\xf5',
    b'\xbc', b'\x44', b'\xe1', b'\x0b', b'\x26', b'\x59', b'\x05', b'\x4d',
    b'\xa9', b'\xee', b'\xa8', b'\xb7', b'\x07', b'\xf5', b'\x02', b'\x4a',
    b'\xe3', b'\xb2', b'\x35', b'\xb0', b'\xaf', b'\x5b', b'\xe3', b'\xad',
    b'\x8e', b'\xd0', b'\xce', b'\x3f', b'\xb1', b'\x8e', b'\x30', b'\x3b',
    b'\x96', b'\xb9', b'\xcd', b'\x09', b'\x22', b'\x10', b'\x5b', b'\xcd',
    b'\xbc', b'\x20', b'\x11', b'\xb2', b'\xf5', b'\x9b', b'\x5a', b'\x5f',
    b'\x23', b'\xdc', b'\x73', b'\x5b', b'\xe4', b'\x4d', b'\x48', b'\x52',
    b'\xb3', b'\x76', b'\x9e', b'\x76', b'\x88', b'\xfc', b'\x14', b'\xd3',
    b'\x99', b'\x18', b'\xd8', b'\xb2', b'\xc7', b'\xf4', b'\x15', b'\xc8',
    b'\x61', b'\xad', b'\x27', b'\x10', b'\x7b', b'\x36', b'\x93', b'\xb9',
    b'\xe2', b'\xc9', b'\x25', b'\x00', b'\xcd', b'\xa1', b'\x9a', b'\x2f',
    b'\xdd', b'\xba', b'\x95', b'\x26', b'\x88', b'\xf2', b'\x39', b'\xf7',
    b'\xb3', b'\x75', b'\x18', b'\x92', b'\xc1', b'\xf7', b'\x19', b'\x79',
    b'\xe2', b'\x00', b'\xed', b'\xdd', b'\x81', b'\x15', b'\xfe', b'\xdf',
    b'\x7d', b'\x2b', b'\x1d', b'\xe3', b'\x20', b'\x3a', b'\x49', b'\xb8',
    b'\x25', b'\xd7', b'\x13', b'\x55', b'\x4d', b'\xdb', b'\x94', b'\xaf',
    b'\x7f', b'\x90', b'\xa4', b'\x3b', b'\xd7', b'\x13', b'\x0e', b'\xe4',
    b'\xe2', b'\x35', b'\xf6', b'\x59', b'\x8d', b'\x54', b'\x86', b'\x25',
    b'\x0c', b'\x06', b'\x31', b'\x28', b'\xe2', b'\xc2', b'\x0c', b'\x82',
    b'\xd7', b'\x75', b'\x25', b'\x6f', b'\x4f', b'\x85', b'\x12', b'\x0d',
    b'\xc4', b'\x98', b'\xbf', b'\xd0', b'\xc7', b'\x08', b'\xd4', b'\x90',
    b'\xbb', b'\x37', b'\x18', b'\x48', b'\x01', b'\xeb', b'\x5e', b'\xa3',
    b'\x67', b'\x0a', b'\x0c', b'\x4f', b'\x06', b'\xa1', b'\x4c', b'\xbb',
    b'\xb2', b'\xab', b'\xf9', b'\x18', b'\xa1', b'\x38', b'\xee', b'\x43',
    b'\xbb', b'\x09', b'\xa2', b'\x95', b'\x9e', b'\xed', b'\xb2', b'\x8b',
    b'\x33', b'\x01', b'\x4d', b'\x54', b'\x7e', b'\xcf', b'\x8b', b'\xaa',
    b'\x87', b'\x18', b'\xc3', b'\xd8', b'\x51', b'\xc8', b'\x6a', b'\x02',
    b'\x8a', b'\x5c', b'\x3e', b'\xf2', b'\xa1', b'\xcf', b'\x30', b'\xbc',
    b'\xee', b'\x34', b'\x45', b'\x22', b'\x64', b'\x6a', b'\x39', b'\xcf',
    b'\xd6', b'\x19', b'\x8f', b'\x28', b'\x54', b'\xb6', b'\x49', b'\xec',
    b'\xa3', b'\xcf', b'\x60', b'\x49', b'\xd6', b'\x03', b'\xcb', b'\x4f',
    b'\xb7', b'\x5b', b'\x35', b'\x5d', b'\x64', b'\x41', b'\x4a', b'\x82',
    b'\x61', b'\x97', b'\x65', b'\x4b', b'\x05', b'\x08', b'\x1f', b'\x06',
    b'\xe2', b'\x67', b'\x5e', b'\xbe', b'\x64', b'\x48', b'\x63', b'\x35',
    b'\xd1', b'\xb8', b'\xd8', b'\xa5', b'\x77', b'\xdf', b'\x10', b'\x21',
    b'\xb1', b'\x3d', b'\xbe', b'\xc4', b'\xc6', b'\xfb', b'\x5f', b'\x25',
    b'\x2b', b'\x0e', b'\x5a', b'\x53', b'\x30', b'\x55', b'\x55', b'\x23',
    b'\xa9', b'\x01', b'\xbd', b'\xe4', b'\xd3', b'\x93', b'\xe5', b'\x56',
    b'\x74', b'\x16', b'\xe2', b'\x14', b'\x33', b'\x62', b'\xb4', b'\x03',
    b'\x07', b'\x8e', b'\xb9', b'\x85', b'\x37', b'\x1e', b'\xa7', b'\xb7',
    b'\x4a', b'\xd8', b'\xb4', b'\xf6', b'\x85', b'\xaf', b'\x0d', b'\x8f',
    b'\xee', b'\x4b', b'\xef', b'\x4d', b'\xbb', b'\x1c', b'\x8c', b'\x3b',
    b'\x12', b'\x05', b'\x0a', b'\x3b', b'\x26', b'\xce', b'\x80', b'\x30',
    b'\x4d', b'\x30', b'\x31', b'\x30', b'\x0d', b'\x06', b'\x09', b'\x60',
    b'\x86', b'\x48', b'\x01', b'\x65', b'\x03', b'\x04', b'\x02', b'\x01',
    b'\x05', b'\x00', b'\x04', b'\x20', b'\xd3', b'\xa3', b'\x58', b'\x58',
    b'\xe0', b'\xf2', b'\xd7', b'\xb4', b'\x1d', b'\x0a', b'\x02', b'\x00',
    b'\x67', b'\x94', b'\x56', b'\x70', b'\xbb', b'\xf2', b'\x16', b'\xea',
    b'\xcc', b'\xe6', b'\x3b', b'\x4c', b'\xeb', b'\xaa', b'\x2f', b'\x6b',
    b'\x31', b'\xa0', b'\x4e', b'\x38', b'\x04', b'\x14', b'\xb7', b'\xee',
    b'\x15', b'\x0c', b'\x4e', b'\xff', b'\x25', b'\x81', b'\xe6', b'\x15',
    b'\xf2', b'\xb7', b'\x91', b'\x30', b'\x3d', b'\x8c', b'\xa0', b'\x0a',
    b'\xdc', b'\x48', b'\x02', b'\x02', b'\x27', b'\x10',
];

impl<'ctx> Build<'ctx> {
    fn new(
        config: &'ctx config::Config,
        metadata: &'ctx cargo::Metadata,
        platform: &'ctx config::ConfigPlatform,
        android: &'ctx config::ConfigPlatformAndroid,
        build_dir: &'ctx std::path::Path,
    ) -> Self {
        // Prepare build directory paths
        let v_apk_dir = build_dir.join("apk");
        let v_artifact_dir = build_dir.join("artifacts");
        let v_class_dir = build_dir.join("classes");
        let v_dex_dir = build_dir.join("dex");
        let v_java_dir = build_dir.join("java");
        let v_resource_dir = build_dir.join("resources");

        // Prepare artifact file paths
        let v_apk_aligned_file = v_artifact_dir.join("package-aligned.apk");
        let v_apk_base_file = v_artifact_dir.join("package-base.apk");
        let v_apk_linked_file = v_artifact_dir.join("package-linked.apk");
        let v_apk_signed_file = v_artifact_dir.join("package-signed.apk");
        let v_classes_dex_file = v_dex_dir.join("classes.dex");
        let v_debug_keystore_file = v_artifact_dir.join("debug.keystore");
        let v_manifest_file = v_artifact_dir.join("AndroidManifest.xml");

        Self {
            android: android,
            build_dir: build_dir,
            config: config,
            metadata: metadata,
            platform: platform,

            apk_dir: v_apk_dir,
            artifact_dir: v_artifact_dir,
            class_dir: v_class_dir,
            dex_dir: v_dex_dir,
            java_dir: v_java_dir,
            resource_dir: v_resource_dir,

            apk_aligned_file: v_apk_aligned_file,
            apk_base_file: v_apk_base_file,
            apk_linked_file: v_apk_linked_file,
            apk_signed_file: v_apk_signed_file,
            classes_dex_file: v_classes_dex_file,
            debug_keystore_file: v_debug_keystore_file,
            manifest_file: v_manifest_file,
        }
    }

    fn generate_manifest(&self) -> String {
        format!(
            concat!(
                r#"<?xml version="1.0" encoding="utf-8"?>"#, "\n",
                r#"<manifest"#, "\n",
                r#"    xmlns:android="http://schemas.android.com/apk/res/android""#, "\n",
                r#"    xmlns:tools="http://schemas.android.com/tools""#, "\n",
                r#"    package="com.example""#, "\n",
                r#">"#, "\n",
                r#"    <application"#, "\n",
                r#"        android:allowBackup="true""#, "\n",
                r#"        android:supportsRtl="true""#, "\n",
                r#"        tools:targetApi="31">"#, "\n",
                r#"        <activity"#, "\n",
                r#"            android:name=".MainActivity""#, "\n",
                r#"            android:exported="true">"#, "\n",
                r#"            <intent-filter>"#, "\n",
                r#"                <action android:name="android.intent.action.MAIN" />"#, "\n",
                r#"                <category android:name="android.intent.category.LAUNCHER" />"#, "\n",
                r#"            </intent-filter>"#, "\n",
                r#"        </activity>"#, "\n",
                r#"    </application>"#, "\n",
                r#"</manifest>"#, "\n",
            ),
        )
    }

    fn prepare(&self) -> Result<(), op::BuildError> {
        // Create build root
        op::mkdir(self.build_dir)?;

        // Create build directories
        op::mkdir(self.apk_dir.as_path())?;
        op::mkdir(self.artifact_dir.as_path())?;
        op::mkdir(self.class_dir.as_path())?;
        op::mkdir(self.dex_dir.as_path())?;
        op::mkdir(self.java_dir.as_path())?;
        op::mkdir(self.resource_dir.as_path())?;

        // Emerge configuration files
        op::update_file(
            self.debug_keystore_file.as_path(),
            &DEBUG_KEYSTORE,
        )?;
        op::update_file(
            self.manifest_file.as_path(),
            self.generate_manifest().as_bytes(),
        )?;

        Ok(())
    }

    fn direct(&self) -> Result<Direct, op::BuildError> {
        let android_home = match std::env::var_os("ANDROID_HOME") {
            None => Err(BuildError::NoAndroidHome),
            Some(v) => Ok(v),
        }?;
        let v_sdk = match sdk::Sdk::new(std::path::Path::new(&android_home)) {
            Ok(v) => Ok::<_, op::BuildError>(v),
            Err(sdk::SdkError::NoSdk(v)) => Err(BuildError::NoSdk(v).into()),
            Err(sdk::SdkError::InvalidSdk(v)) => Err(BuildError::InvalidSdk(v).into()),
            Err(v) => Err(lib::error::Uncaught::box_debug(v).into()),
        }?;
        let v_ndk = match v_sdk.ndk(None) {
            Ok(v) => Ok::<_, op::BuildError>(v),
            Err(sdk::SdkError::NoNdk) => Err(BuildError::NoNdk.into()),
            Err(sdk::SdkError::InvalidNdk(v)) => Err(BuildError::InvalidNdk(v).into()),
            Err(v) => Err(lib::error::Uncaught::box_debug(v).into()),
        }?;
        let v_build_tools = match v_sdk.build_tools(None) {
            Ok(v) => Ok::<_, op::BuildError>(v),
            Err(sdk::SdkError::NoBuildTools) => Err(BuildError::NoBuildTools.into()),
            Err(sdk::SdkError::InvalidBuildTools(v)) => Err(BuildError::InvalidBuildTools(v).into()),
            Err(v) => Err(lib::error::Uncaught::box_debug(v).into()),
        }?;
        let v_platform = match v_sdk.platform(self.android.min_sdk) {
            Ok(v) => Ok::<_, op::BuildError>(v),
            Err(sdk::SdkError::NoPlatform(v)) => Err(BuildError::NoPlatform(v).into()),
            Err(sdk::SdkError::InvalidPlatform(v)) => Err(BuildError::InvalidPlatform(v).into()),
            Err(v) => Err(lib::error::Uncaught::box_debug(v).into()),
        }?;
        let v_platform_jar = v_platform.as_path().join("android.jar");
        let v_jdk = match sdk::Jdk::new(None).map_err(|v| *v) {
            Ok(v) => Ok::<_, op::BuildError>(v),
            Err(sdk::JdkError::NoJdk(v)) => Err(BuildError::NoJdk(v).into()),
            Err(sdk::JdkError::InvalidJdk(v)) => Err(BuildError::InvalidJdk(v).into()),
        }?;
        let v_kdk = match sdk::Kdk::new(None).map_err(|v| *v) {
            Ok(v) => Ok::<_, op::BuildError>(v),
            Err(sdk::KdkError::NoKdk(v)) => Err(BuildError::NoKdk(v).into()),
            Err(sdk::KdkError::InvalidKdk(v)) => Err(BuildError::InvalidKdk(v).into()),
        }?;

        Ok(Direct {
            build: self,

            build_tools: v_build_tools,
            jdk: v_jdk,
            kdk: v_kdk,
            ndk: v_ndk,
            platform: v_platform,
            platform_jar: v_platform_jar,
            sdk: v_sdk,
        })
    }
}

impl<'ctx> Direct<'ctx> {
    fn build_resources(
        &self,
    ) -> Result<(bool, Vec<std::path::PathBuf>), op::BuildError> {
        let mut res_files: BTreeMap::<std::path::PathBuf, std::path::PathBuf>;
        let mut new = false;

        // Collect all resource files to be compiled. We get a list of
        // resource directories. Each of these contains a list of resource
        // type directories, which then each contains resource files. Any
        // stray entries are silently ignored.
        res_files = BTreeMap::new();
        for rdir in self.build.metadata.android_sets.iter()
            .map(|v| v.resource_dirs.iter())
            .flatten()
        {
            let sdirs = std::fs::read_dir(rdir).map_err(
                |_| op::BuildError::DirectoryTraversal(rdir.into()),
            )?;
            for sdir_iter in sdirs {
                let sdir_entry = sdir_iter.map_err(
                    |_| op::BuildError::DirectoryTraversal(rdir.into()),
                )?;
                let sdir = &sdir_entry.path();

                if !sdir.is_dir() {
                    continue;
                }

                let tdirs = std::fs::read_dir(sdir).map_err(
                    |_| op::BuildError::DirectoryTraversal(sdir.into()),
                )?;
                for tdir_iter in tdirs {
                    let tdir_entry = tdir_iter.map_err(
                        |_| op::BuildError::DirectoryTraversal(sdir.into()),
                    )?;
                    let tdir = &tdir_entry.path();

                    if !tdir.is_dir() {
                        // Compute the output file name. Note that this cannot
                        // fail here, since its only failure condition is when
                        // an invalid path, or a path without directory is given.
                        // We just iterated a directory, so both must be set.
                        let out = flatres::Query::output_file_name(
                            tdir,
                        ).ok_or_else(
                            || -> op::BuildError {
                                lib::error::Uncaught::box_any(()).into()
                            },
                        )?;

                        res_files.insert(
                            self.build.resource_dir.join(out),
                            tdir.into(),
                        );
                    }
                }
            }
        }

        // For each resource file, check whether the target file exists and is
        // newer than the source. In this case, skip compilation. Otherwise,
        // invoke the Android flat-resource compiler.
        for (to, from) in &res_files {
            let to_mod = to.metadata().ok()
                .map(|v| v.modified().ok())
                .flatten();
            let from_mod = from.metadata().ok()
                .map(|v| v.modified().ok())
                .flatten();
            if let (Some(dst), Some(src)) = (to_mod, from_mod) {
                if src < dst {
                    continue;
                }
            }

            let query = flatres::Query {
                build_tools: self.build_tools.clone(),
                output_dir: self.build.resource_dir.clone(),
                resource_file: from.clone(),
            };

            query.run().map_err(|v| -> op::BuildError {
                match v {
                    flatres::Error::Exec(v) => BuildError::FlatresExec(v).into(),
                    flatres::Error::Exit(v) => BuildError::FlatresExit(v).into(),
                    v => lib::error::Uncaught::box_debug(v).into(),
                }
            })?;

            new = true;
        }

        Ok((new, res_files.into_keys().collect()))
    }

    fn build_apk(
        &self,
        resources: &(bool, Vec<std::path::PathBuf>),
    ) -> Result<bool, op::BuildError> {
        let mut link_files = Vec::new();
        link_files.push(self.platform_jar.clone());

        let query = apk::LinkQuery {
            build_tools: self.build_tools.clone(),
            asset_dirs: Vec::new(),
            link_files: link_files,
            manifest_file: self.build.manifest_file.clone(),
            output_file: self.build.apk_base_file.clone(),
            output_java_dir: Some(self.build.java_dir.clone()),
            resource_files: resources.1.clone(),
        };

        query.run().map_err(|v| -> op::BuildError {
            match v {
                apk::LinkError::Exec(v) => BuildError::FlatresExec(v).into(),
                apk::LinkError::Exit(v) => BuildError::FlatresExit(v).into(),
            }
        })?;

        Ok(true)
    }

    fn build_java(
        &self,
    ) -> Result<bool, op::BuildError> {
        let mut sources = Vec::new();

        // Collect all Java sources. This includes all Java sources specified
        // by the package dependencies, but also the build-generated Java files
        // like the Android `R` class.
        // Since other JVM-languages might use the same source-tree, filter
        // files by their `*.java` extension.

        sources.append(&mut op::lsrdir(self.build.java_dir.as_path())?);

        for set in &self.build.metadata.android_sets {
            for dir in &set.java_dirs {
                sources.append(&mut op::lsrdir(dir.as_path())?);
            }
        }

        sources.retain(|v| v.extension() == Some(std::ffi::OsStr::new("java")));

        if sources.is_empty() {
            return Ok(false);
        }

        // Run the Java compiler. We never try to optimize this and check
        // whether files are up-to-date. Java has a non-trivial source->target
        // file correlation, which is out of scope for us. Just let the
        // compiler deal with it.

        let query = java::Query {
            class_paths: &[&self.platform_jar, &self.build.class_dir],
            jdk: &self.jdk,
            output_dir: &self.build.class_dir,
            source_files: &sources,
        };

        query.run().map_err(|v| -> op::BuildError {
            match v {
                java::Error::UnsupportedPath(v) => BuildError::UnsupportedPath(v).into(),
                java::Error::Exec(v) => BuildError::JavacExec(v).into(),
                java::Error::Exit(v) => BuildError::JavacExit(v).into(),
            }
        })?;

        Ok(true)
    }

    fn build_kotlin(
        &self,
    ) -> Result<bool, op::BuildError> {
        let mut sources = Vec::new();

        // Collect all Kotlin sources. This includes all sources specified by
        // the package dependencies. Note that several JVM-languages might
        // share a source tree, so we filter by the `*.kt` file extension.

        for set in &self.build.metadata.android_sets {
            for dir in &set.kotlin_dirs {
                sources.append(&mut op::lsrdir(dir.as_path())?);
            }
        }
        sources.retain(|v| v.extension() == Some(std::ffi::OsStr::new("kt")));

        if sources.is_empty() {
            return Ok(false);
        }

        // Invoke the Kotlin compiler. Similar to the Java compiler, the
        // source->target file correlation is hard to predict without parsing
        // Java code. Hence, never try to perform timestamp checks and instead
        // let the compiler deal with incremental compilation.

        let query = kotlin::Query {
            class_paths: &[&self.platform_jar, &self.build.class_dir],
            kdk: &self.kdk,
            output_dir: &self.build.class_dir,
            source_files: &sources,
        };

        query.run().map_err(|v| -> op::BuildError {
            match v {
                kotlin::Error::UnsupportedPath(v) => BuildError::UnsupportedPath(v).into(),
                kotlin::Error::Exec(v) => BuildError::KotlincExec(v).into(),
                kotlin::Error::Exit(v) => BuildError::KotlincExit(v).into(),
            }
        })?;

        Ok(true)
    }

    fn build_dex(
        &self,
    ) -> Result<bool, op::BuildError> {
        let mut sources = op::lsrdir(self.build.class_dir.as_path())?;
        sources.retain(|v| v.extension() == Some(std::ffi::OsStr::new("class")));

        if sources.is_empty() {
            return Ok(false);
        }

        let query = dex::Query {
            api: Some(self.build.android.min_sdk),
            build_tools: &self.build_tools,
            class_paths: &Vec::<std::path::PathBuf>::new(),
            debug: false,
            libs: &[&self.platform_jar],
            output_dir: &self.build.dex_dir,
            source_files: &sources,
        };

        query.run().map_err(|v| -> op::BuildError {
            match v {
                dex::Error::Exec(v) => BuildError::DexExec(v).into(),
                dex::Error::Exit(v) => BuildError::DexExit(v).into(),
            }
        })?;

        Ok(true)
    }

    fn build_cargo(
        &self,
    ) -> Result<(bool, BTreeMap<String, cargo::Build>), op::BuildError> {
        let mut res = BTreeMap::new();

        // Android SDKs ship prebuilt toolchains for `x86_64` on linux, macos
        // and windows. However, you can likely call it from `x86` (given the
        // OS runs in 64-bit mode), or on macos via emulators.
        //
        // XXX: The target platform of the calling binary does not have to
        //      match the host-os, nor does it prevent emulators from running
        //      foreign-platform SDKs. We should support selecting the host-os
        //      via cmdline.
        let host = if cfg!(
            all(
                target_os = "linux",
                any(
                    target_arch = "x86",
                    target_arch = "x86_64",
                ),
            ),
        ) {
            "linux-x86_64"
        } else if cfg!(
            all(
                target_os = "macos",
                any(
                    target_arch = "aarch64",
                    target_arch = "x86",
                    target_arch = "x86_64",
                ),
            ),
        ) {
            "darwin-x86_64"
        } else if cfg!(
            all(
                target_os = "windows",
                any(
                    target_arch = "x86",
                    target_arch = "x86_64",
                ),
            ),
        ) {
            "windows-x86_64"
        } else {
            return Err(BuildError::UnsupportedHost.into());
        };

        for abi in &self.build.android.abis {
            let (target, linker_env, linker_prefix) = match abi.as_str() {
                "armeabi-v7a" => Ok((
                    "armv7-linux-androideabi",
                    "CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER",
                    "armv7a-linux-androideabi",
                )),
                "arm64-v8a" => Ok((
                    "aarch64-linux-android",
                    "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
                    "aarch64-linux-android",
                )),
                "x86" => Ok((
                    "i686-linux-android",
                    "CARGO_TARGET_I686_LINUX_ANDROID_LINKER",
                    "i686-linux-android",
                )),
                "x86_64" => Ok((
                    "x86_64-linux-android",
                    "CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER",
                    "x86_64-linux-android",
                )),
                v => Err(BuildError::UnsupportedAbi(v.into())),
            }?;
            let linker_bin = format!(
                "toolchains/llvm/prebuilt/{}/bin/{}{}-clang",
                host,
                linker_prefix,
                self.build.android.min_sdk,
            );
            let linker_path = self.ndk.root().join(linker_bin);

            let query = cargo::BuildQuery {
                default_features: true,
                envs: vec![(linker_env.into(), linker_path.into())],
                features: Vec::new(),
                profile: None,
                target: Some(target.into()),
                workspace: self.build.config.path_application.clone(),
            };

            let build = query.run().map_err(
                |v| -> op::BuildError { v.into() },
            )?;

            res.insert(abi.into(), build);
        }

        Ok((true, res))
    }

    fn link_apk(
        &self,
        bins: &(bool, BTreeMap<String, cargo::Build>),
    ) -> Result<bool, op::BuildError> {
        let mut path: std::path::PathBuf = self.build.apk_dir.clone();
        let mut add = Vec::new();

        // Copy the intermediate APK to avoid operating on intermediates
        // multiple times, and thus modifying timestamps needlessly.
        op::copy_file(&self.build.apk_base_file, &self.build.apk_linked_file)?;

        // Assemble the APK directory. Since `aapt` retains paths verbatim
        // in the APK, we need to assemble a directory with the exact
        // contents we want in the APK. Unfortunately, this means copying
        // our artifacts to the desired sub-paths in the APK assembly
        // directory.

        path.push("classes.dex");
        add.push("classes.dex".into());
        op::copy_file(&self.build.classes_dex_file, path.as_path())?;
        path.pop();

        path.push("lib");
        op::mkdir(path.as_path())?;
        for (abi, set) in &bins.1 {
            path.push(abi);
            op::mkdir(path.as_path())?;
            for v in &set.artifacts {
                let file_name = std::path::Path::new(v)
                    .file_name()
                    .expect("Cargo artifact has no file-name");

                let mut lib_path = std::ffi::OsString::new();
                lib_path.push(format!("lib/{}/", abi));
                lib_path.push(file_name);

                path.push(file_name);
                add.push(lib_path.into());
                op::copy_file(std::path::Path::new(v), path.as_path())?;
                path.pop();
            }
            path.pop();
        }
        path.pop();

        // Now invoke the `aapt` tools to alter and thus link the final
        // APK. Preferably, we would just invoke a standard ZIP tool, but
        // they are not packaged with the Android SDK, and thus would mean
        // we have another build-time dependency.
        //
        // XXX: Ideally, we would ship, or depend, on a simple ZIP archive
        //      builder in pure Rust, and thus avoid all this dance.

        let query = apk::AlterQuery {
            base_dir: Some(self.build.apk_dir.clone()),
            build_tools: self.build_tools.clone(),
            add_files: add,
            apk_file: self.build.apk_linked_file.clone(),
        };

        query.run().map_err(|v| -> op::BuildError {
            match v {
                apk::AlterError::Exec(v) => BuildError::AaptExec(v).into(),
                apk::AlterError::Exit(v) => BuildError::AaptExit(v).into(),
            }
        })?;

        // Since APKs are normal zip-files, and those have no alignment
        // restrictions, we have to align the file explicitly to ensure
        // Android can run it directly.

        let query = apk::AlignQuery {
            build_tools: self.build_tools.clone(),
            input_file: self.build.apk_linked_file.clone(),
            output_file: self.build.apk_aligned_file.clone(),
        };

        query.run().map_err(|v| -> op::BuildError {
            match v {
                apk::AlignError::Exec(v) => op::BuildError::Exec("zipalign".into(), v),
                apk::AlignError::Exit(v) => op::BuildError::Exit("zipalign".into(), v),
            }
        })?;

        Ok(true)
    }
}

fn build_direct(
    build: &Build,
) -> Result<(), op::BuildError> {
    let direct = build.direct()?;

    eprintln!("Compile Android resources..");
    let res = direct.build_resources()?;

    eprintln!("Build Android APK..");
    direct.build_apk(&res)?;

    eprintln!("Compile Android Java sources..");
    direct.build_java()?;

    eprintln!("Compile Android Kotlin sources..");
    direct.build_kotlin()?;

    eprintln!("Build DEX files..");
    direct.build_dex()?;

    eprintln!("Build Cargo package..");
    let bins = direct.build_cargo()?;

    eprintln!("Link APK..");
    direct.link_apk(&bins)?;

    Ok(())
}

pub fn build(
    config: &config::Config,
    metadata: &cargo::Metadata,
    platform: &config::ConfigPlatform,
    android: &config::ConfigPlatformAndroid,
    build_dir: &std::path::Path,
) -> Result<(), op::BuildError> {
    let build = Build::new(
        config,
        metadata,
        platform,
        android,
        build_dir,
    );

    build.prepare()?;
    build_direct(&build)?;

    Ok(())
}
