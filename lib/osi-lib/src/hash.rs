//! # Hashing Support
//!
//! This modules provides access to standard hashing functions with streaming
//! support. A common trait is provided to allow uniform access regardless of
//! the hashing function used.

pub mod sha256;

/// ## Hash Engine
///
/// A hash engine can be used to stream data into a hashing function and
/// produce the final hash value. A single engine can be reused for multiple
/// hash operations.
///
/// Hash engines operate on an input byte stream and produce a fixed hash
/// type (usually a fixed-size byte array).
pub trait Engine {
    /// ## Hash Result
    ///
    /// This associated type represents the result of a hashing function.
    /// In most cases it is a fixed size byte array. See `Hash` for a
    /// common type used for this.
    type Hash;

    /// ## Check whether the Engine is reset
    ///
    /// Return whether the engine is currently reset, or whether it has
    /// data queued or processed.
    fn is_reset(&self) -> bool;

    /// ## Reset Engine
    ///
    /// Reset the engine to its initial state. This allows discarding an
    /// ongoing streaming operation without having to recreate the engine.
    ///
    /// Note that an engine is automatically reset on initialization and
    /// after every finalization. There is no need to manually reset the
    /// engine in these situations.
    fn reset(&mut self);

    /// ## Push Data into the Engine
    ///
    /// Push the given bytes into the engine. In most cases, the engine will
    /// buffer data up to a fixed limit before processing it.
    fn push(&mut self, data: &[u8]);

    /// ## Push Zeroes into the Engine
    ///
    /// Push a given amount of zero-bytes into the engine. This is an
    /// optimization to allow pushing large amounts of zeros without
    /// actually creating the input data.
    fn push_zero(&mut self, mut length: usize) {
        while length > 0 {
            let n = core::cmp::min(length, 128);
            self.push(&[0u8; 128][0..n]);
            length = length - n;
        }
    }

    /// ## Produce Final Hash
    ///
    /// Finalize the streaming operation and produce the final hash for the
    /// entire data that was streamed into the engine.
    ///
    /// The engine is automatically reset after this operation and ready
    /// for the next hashing operation.
    fn finalize(&mut self) -> Self::Hash;
}

/// ## Hash Value
///
/// This represents a possible hash value of most hashing functions. This
/// can be used by the hashing functions to implement their hash type, if
/// the hash is a fixed-size byte array.
#[derive(Clone, Copy, Debug)]
pub struct Hash<const SIZE: usize>(pub [u8; SIZE]);

/// ## Produce Instant Hash
///
/// Produce the hash value of the given bytes. This is a shortcut when
/// a streaming API is not required.
pub fn hash<Backend: Default + Engine>(data: &[u8]) -> Backend::Hash {
    let mut e = <Backend as Default>::default();
    e.push(data);
    e.finalize()
}

impl<const SIZE: usize> Hash<SIZE> {
    /// ## Create from Bytes
    ///
    /// Create a new hash value from its byte representation. This is
    /// equivalent to creating it via `Hash(bytes)`.
    pub fn from_bytes(bytes: &[u8; SIZE]) -> Self {
        Self(*bytes)
    }

    /// ## Return Byte Representation
    ///
    /// Return a reference to the byte representation of the hash value. This
    /// is equivalent to `&h.0`
    pub fn as_bytes(&self) -> &[u8; SIZE] {
        &self.0
    }

    /// ## Produce Hex-String Representation
    ///
    /// Format the hash as a hex-string. That is, produce a string with only
    /// the characters '0'-'9' and 'a'-'f'. Each character represents 4-bits
    /// of the hash value (in big-endian order).
    ///
    /// Due to limitations of the const-evaluation of Rust, this returns a
    /// `String` rather than `[char; SIZE * 2]`.
    pub fn to_hex(&self) -> alloc::string::String {
        fn digit_to_char(digit: u8) -> char {
            match digit {
                0x0 => '0', 0x1 => '1', 0x2 => '2', 0x3 => '3',
                0x4 => '4', 0x5 => '5', 0x6 => '6', 0x7 => '7',
                0x8 => '8', 0x9 => '9', 0xa => 'a', 0xb => 'b',
                0xc => 'c', 0xd => 'd', 0xe => 'e', 0xf => 'f',
                _ => panic!("Invalid digit input"),
            }
        }

        let mut s = alloc::string::String::with_capacity(
            SIZE.checked_mul(2).unwrap(),
        );

        for b in self.0 {
            s.push(digit_to_char((b >> 4) & 0x0f));
            s.push(digit_to_char((b >> 0) & 0x0f));
        }

        s
    }
}

impl<const SIZE: usize> Default for Hash<SIZE> {
    fn default() -> Self {
        Self([0u8; SIZE])
    }
}

impl<const SIZE: usize> From<&[u8; SIZE]> for Hash<SIZE> {
    fn from(v: &[u8; SIZE]) -> Self {
        Self::from_bytes(v)
    }
}
