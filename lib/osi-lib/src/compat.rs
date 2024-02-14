//! # Standard Library Compatibility
//!
//! This module provides compatibility types that allow easy integration for
//! situations where the Standard Library is available.

/// Compatibility type to bridge between `std::io::Write` and
/// `core::fmt::Write`.
///
/// This type can be used to wrap any type that implements `std::io::Write` to
/// get a wrapping type that implements `core::fmt::Write`. The standard
/// library does not provide this blanket implementation due to backwards
/// compatibility reasons.
#[repr(transparent)]
pub struct Write<T>(pub T);

#[cfg(feature = "std")]
impl<T> core::fmt::Write for Write<T>
where
    T: std::io::Write
{
    fn write_fmt(&mut self, args: core::fmt::Arguments) -> core::fmt::Result {
        self.0.write_fmt(args).map_err(|_| core::fmt::Error)
    }

    fn write_str(&mut self, v: &str) -> core::fmt::Result {
        self.0.write_all(v.as_bytes()).map_err(|_| core::fmt::Error)
    }
}

/// Compatibility type for `std::ffi::OsStr`. This type represents the same
/// value as returned by `std::ffi::OsStr::as_encoded_bytes()` for a given
/// `OsStr` value.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct OsStr {
    inner: [u8],
}

impl OsStr {
    /// Create an `OsStr` compatibility type from the encoded-bytes
    /// representation of `std::ffi::OsStr`.
    ///
    /// ## Safety
    ///
    /// The source must be either valid UTF-8, or obtained via
    /// `std::ffi::OsStr::as_encoded_bytes()` (see its documentation for
    /// details on the allowed operations).
    pub unsafe fn from_encoded_bytes_unchecked(v: &[u8]) -> &Self {
        core::mem::transmute(v)
    }

    /// Create an `OsStr` compatibility type from its `std` equivalent.
    /// Both have the same representation, so the conversion is free.
    #[cfg(feature = "std")]
    pub fn from_osstr(v: &std::ffi::OsStr) -> &Self {
        unsafe {
            // SAFETY: Both definitions of `encoded-bytes` match, so
            //         this conversion is allowed.
            Self::from_encoded_bytes_unchecked(
                v.as_encoded_bytes(),
            )
        }
    }

    /// Create an `OsStr` compatibility type from a valid UTF-8 string
    /// given as Rust `str`.
    pub fn from_str(v: &str) -> &Self {
        unsafe {
            Self::from_encoded_bytes_unchecked(v.as_bytes())
        }
    }

    /// Create an `OsStr` compatibility type from a byte buffer, yielding
    /// an error if it is not valid UTF-8.
    pub fn from_utf8(v: &[u8]) -> Result<&Self, core::str::Utf8Error> {
        Ok(Self::from_str(core::str::from_utf8(v)?))
    }

    /// Return the encoded-bytes representation of the compatibility type.
    /// This is equivalent to `std::ffi::OsStr::as_encoded_bytes()`.
    pub fn as_encoded_bytes(&self) -> &[u8] {
        &self.inner
    }

    /// Return the `std` equivalent for this compatibility type. Both have the
    /// same representation, so the conversion is free.
    #[cfg(feature = "std")]
    pub fn as_osstr(&self) -> &std::ffi::OsStr {
        unsafe {
            // SAFETY: Both definitions of `encoded-bytes` match, so
            //         this conversion is allowed.
            std::ffi::OsStr::from_encoded_bytes_unchecked(
                self.as_encoded_bytes(),
            )
        }
    }

    /// Return a Rust `str` for the value of this compatibility type. This is
    /// only possible of the data is valid UTF-8. Hence, this requires a UTF-8
    /// check and returns its errors, if any.
    pub fn to_str(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(self.as_encoded_bytes())
    }

    /// Return a Rust string for the value of this compatibility type. This
    /// will replace invalid Unicode sequences with the Unicode replacement
    /// character. See `alloc::string::String::from_utf8_lossy()` for details.
    pub fn to_string_lossy(&self) -> alloc::borrow::Cow<str> {
        alloc::string::String::from_utf8_lossy(self.as_encoded_bytes())
    }
}

impl<'a> From<&'a str> for &'a OsStr {
    fn from(v: &'a str) -> &'a OsStr {
        OsStr::from_str(v)
    }
}

#[cfg(feature = "std")]
impl<'a> From<&'a std::ffi::OsStr> for &'a OsStr {
    fn from(v: &'a std::ffi::OsStr) -> &'a OsStr {
        OsStr::from_osstr(v)
    }
}

#[cfg(feature = "std")]
impl AsRef<std::ffi::OsStr> for OsStr {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.as_osstr()
    }
}
