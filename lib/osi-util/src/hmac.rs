//! # HMAC Support
//!
//! This module implements `HMAC` (Keyed-Hashing for Message Authentication,
//! [RFC 2104]).

/// Hmac Engine
///
/// This type represents an Hmac engine. It takes the chunk size of the
/// hash engine, as well as the hash engine itself as type arguments.
///
/// A single Hmac engine is tied to a given key, but can be used to calculate
/// message authentication codes (MACs) for multiple different messages.
pub struct Hmac<const CHUNK: usize, Engine> {
    engine: Engine,
    ipad: [u8; CHUNK],
    opad: [u8; CHUNK],
}

/// Hmac Engine with SHA-256
pub type HmacSha256 = Hmac<64, crate::hash::sha256::Engine>;

impl<const HASH: usize, const CHUNK: usize, Engine> Hmac<CHUNK, Engine>
where
    Engine: crate::hash::Engine<Hash = crate::hash::Hash<HASH>>,
{
    /// ## Create with given Engine
    ///
    /// Create a new Hmac engine with the given hash engine. The hash
    /// engine must be reset.
    ///
    /// The secret key for the Hmac engine must be provided as `key` and
    /// will be retained by the Hmac engine for processing.
    pub fn with_engine(key: &[u8], engine: Engine) -> Self {
        let t_ukey;

        // Passing a used engine is undefined. Ensure the caller only passes
        // known state to us.
        assert!(engine.is_reset());

        // Create the HMAC engine and prepare the inner and outer pads with
        // their respective defaults.
        let mut hmac = Self {
            engine: engine,
            ipad: [0x36u8; CHUNK],
            opad: [0x5cu8; CHUNK],
        };

        // Create a shortened key with a maximum length of `CHUNK`. So if
        // the key is too long, it is simply hashed with the same engine
        // and then padded, in case the length of the hash is shorter than
        // the length of the chunk for this given hashing function.
        let ukey = if key.len() > CHUNK {
            hmac.engine.push(key);
            t_ukey = hmac.engine.finalize();
            t_ukey.as_bytes()
        } else {
            key
        };

        // Copy the shortened key into the inner and outer pads, applying
        // their default modifier respectively.
        for (i, v) in ukey.iter().enumerate() {
            hmac.ipad[i] = v ^ 0x36u8;
            hmac.opad[i] = v ^ 0x5cu8;
        }

        hmac
    }

    /// ## Reset Engine
    ///
    /// Reset the engine to its initial state. This allows discarding an
    /// ongoing streaming operation without having to recreate the engine.
    ///
    /// Note that an engine is automatically reset on initialization and
    /// after every finalization. There is no need to manually reset the
    /// engine in these situations.
    pub fn reset(&mut self) {
        self.engine.reset();
    }

    /// ## Push Data into the Engine
    ///
    /// Push the given bytes into the engine. In most cases, the engine will
    /// buffer data up to a fixed limit before processing it.
    pub fn push(&mut self, data: &[u8]) {
        if data.len() > 0 {
            if self.engine.is_reset() {
                self.engine.push(&self.ipad);
            }
            self.engine.push(data);
        }
    }

    /// ## Push Zeroes into the Engine
    ///
    /// Push a given amount of zero-bytes into the engine. This is an
    /// optimization to allow pushing large amounts of zeros without
    /// actually creating the input data.
    pub fn push_zero(&mut self, length: usize) {
        if length > 0 {
            if self.engine.is_reset() {
                self.engine.push(&self.ipad);
            }
            self.engine.push_zero(length)
        }
    }

    /// ## Produce Final Output
    ///
    /// Finalize the streaming operation and produce the final output for the
    /// entire data that was streamed into the engine.
    ///
    /// The engine is automatically reset after this operation and ready
    /// for the next run.
    pub fn finalize(&mut self) -> Engine::Hash {
        if self.engine.is_reset() {
            self.engine.push(&self.ipad);
        }
        let v = self.engine.finalize();

        self.engine.push(&self.opad);
        self.engine.push(v.as_bytes());
        self.engine.finalize()
    }
}

impl<const HASH: usize, const CHUNK: usize, Engine> Hmac<CHUNK, Engine>
where
    Engine: crate::hash::Engine<Hash = crate::hash::Hash<HASH>>,
    Engine: Default,
{
    /// Create a new Hmac Engine
    ///
    /// Create a new Hmac engine with the given key. This creates a new hash
    /// engine and uses it. See `with_engine()` for how to provide the engine
    /// manually.
    pub fn new(key: &[u8]) -> Self {
        Self::with_engine(key, Default::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Perform some basic testing with known values.
    #[test]
    fn basic() {
        let mut e = HmacSha256::new(b"key");

        e.push(b"The quick brown fox jumps over the lazy dog");

        assert_eq!(
            e.finalize().to_hex(),
            "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
        );
    }
}
