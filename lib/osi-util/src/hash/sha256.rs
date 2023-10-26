//! # SHA-256 Hash Function
//!
//! This is an implementation of the SHA-256 hash function, as defined in
//! RFC-6234.

/// ## Size of SHA-256 Hashes
///
/// The SHA-256 engine produces a fixed-size hash as result. This is the size
/// of the hash in bytes.
const SHA256_HASH: usize = 32;

/// ## Size of SHA-256 Engine Chunks
///
/// The SHA-256 engine operates on input chunks of this size. The final chunk
/// is padded by the engine.
const SHA256_CHUNK: usize = 64;

/// ## SHA-256 Engine
///
/// This is the streaming engine for the SHA-256 hashing function. It
/// implements the `crate::hash::Engine` trait for SHA-256.
#[derive(Clone, Debug)]
pub struct Engine {
    hash: [u32; 8],
    chunk: [u8; SHA256_CHUNK],
    remaining: usize,
    total: u64,
}

/// ## Initial SHA-256 Engine Hash
///
/// This is the initial state of the SHA-256 engine as defined in RFC-6234. It
/// corresponds to the first 32 bits of the fractional parts of the square
/// roots of the first 8 primes 2..19.
const SHA256_H: [u32; 8] = [
    0x6a09e667,
    0xbb67ae85,
    0x3c6ef372,
    0xa54ff53a,
    0x510e527f,
    0x9b05688c,
    0x1f83d9ab,
    0x5be0cd19,
];

/// ## SHA-256 Engine Constants
///
/// This is a set of constants mixed into the SHA-256 engine as defined in
/// RFC-6234. It corresponds to the first 32 bits of the fractional parts of
/// the cube roots of the first 64 primes 2..311.
const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// ## Advance the SHA-256 Engine one Step
///
/// This performs a single step of the SHA-256 engine. It takes the `schedule`
/// and current `state` as input and updates the state accordingly.
fn sha256_step(
    schedule: &[u32; 16],
    state: &mut [u32; 8],
    block: usize,
    step: usize,
) {
    assert!(block < 4);
    assert!(step < 16);

    let s0 = (state[0].rotate_right(2))
        ^ (state[0].rotate_right(13))
        ^ (state[0].rotate_right(22));
    let s1 = (state[4].rotate_right(6))
        ^ (state[4].rotate_right(11))
        ^ (state[4].rotate_right(25));
    let maj = (state[0] & state[1])
        ^ (state[0] & state[2])
        ^ (state[1] & state[2]);
    let ch = (state[4] & state[5])
        ^ (!state[4] & state[6]);

    let temp1 = state[7]
        .wrapping_add(s1)
        .wrapping_add(ch)
        .wrapping_add(SHA256_K[(block * 16) + step])
        .wrapping_add(schedule[step]);
    let temp2 = s0.wrapping_add(maj);

    state[7] = state[6];
    state[6] = state[5];
    state[5] = state[4];
    state[4] = state[3].wrapping_add(temp1);
    state[3] = state[2];
    state[2] = state[1];
    state[1] = state[0];
    state[0] = temp1.wrapping_add(temp2);
}

/// ## Advance the SHA-256 Engine one Round
///
/// This performs a full round of the SHA-256 engine. It takes an entire chunk
/// as input data and calculates the state update for a full round (i.e., 64
/// steps).
fn sha256_round(
    data: &[u8; 64],
    hash: &mut [u32; 8],
) {
    let mut state: [u32; 8] = *hash;
    let mut schedule: [u32; 16];

    // Compute first block with unmodified input.
    {
        let schedule_init = |i: usize| {
            u32::from_be_bytes([
                data[i * 4 + 0], data[i * 4 + 1], data[i * 4 + 2], data[i * 4 + 3],
            ])
        };
        schedule = [
            schedule_init( 0), schedule_init( 1), schedule_init( 2), schedule_init( 3),
            schedule_init( 4), schedule_init( 5), schedule_init( 6), schedule_init( 7),
            schedule_init( 8), schedule_init( 9), schedule_init(10), schedule_init(11),
            schedule_init(12), schedule_init(13), schedule_init(14), schedule_init(15),
        ];
        for i in 0..16 {
            sha256_step(&schedule, &mut state, 0, i);
        }
    }

    // Compute remaining 3 blocks with additive input.
    for block in 1..4 {
        for i in 0..16 {
            let s0 = (schedule[(i + 1) % 16].rotate_right(7))
                ^ (schedule[(i + 1) % 16].rotate_right(18))
                ^ (schedule[(i + 1) % 16] >> 3);
            let s1 = (schedule[(i + 14) % 16].rotate_right(17))
                ^ (schedule[(i + 14) % 16].rotate_right(19))
                ^ (schedule[(i + 14) % 16] >> 10);
            schedule[i] = schedule[i]
                .wrapping_add(s0)
                .wrapping_add(schedule[(i + 9) % 16])
                .wrapping_add(s1);
        }
        for i in 0..16 {
            sha256_step(&schedule, &mut state, block, i);
        }
    }

    // Update the hash value with the result of this round.
    for i in 0..hash.len() {
        hash[i] = hash[i].wrapping_add(state[i]);
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            hash: SHA256_H,
            chunk: [0; SHA256_CHUNK],
            remaining: SHA256_CHUNK,
            total: 0,
        }
    }
}

impl crate::hash::Engine for Engine {
    type Hash = crate::hash::Hash::<SHA256_HASH>;

    fn reset(&mut self) {
        self.hash = SHA256_H;
        self.remaining = SHA256_CHUNK;
        self.total = 0;
    }

    fn push(&mut self, mut data: &[u8]) {
        while data.len() > 0 {
            // Copy as much as possible into the remaining space of the
            // stream buffer, and adjust `data` accordingly.
            let n = core::cmp::min(self.remaining, data.len());
            let idx = self.chunk.len().checked_sub(self.remaining).unwrap();

            self.chunk[idx..(idx + n)].copy_from_slice(&data[..n]);
            data = &data[n..];

            self.remaining = self.remaining
                .checked_sub(n).unwrap();
            self.total = self.total
                .checked_add(u64::try_from(n).unwrap()).unwrap();

            // If the stream buffer is full, commit it to the engine.
            if self.remaining == 0 {
                sha256_round(&self.chunk, &mut self.hash);
                self.remaining = self.chunk.len();
            }
        }
    }

    fn push_zero(&mut self, mut length: usize) {
        while length > 0 {
            // Zero as much as possible of the remaining space of the
            // stream buffer.
            let n = core::cmp::min(self.remaining, length);
            let idx = self.chunk.len().checked_sub(self.remaining).unwrap();

            self.chunk[idx..(idx + n)].fill(0);
            length = length.checked_sub(n).unwrap();

            self.remaining = self.remaining
                .checked_sub(n).unwrap();
            self.total = self.total
                .checked_add(u64::try_from(n).unwrap()).unwrap();

            // If the stream buffer is full, commit it to the engine.
            if self.remaining == 0 {
                sha256_round(&self.chunk, &mut self.hash);
                self.remaining = self.chunk.len();
            }
        }
    }

    fn finalize(&mut self) -> Self::Hash {
        // Take a copy of the total number of bytes the caller pushed into the
        // engine. Any further bytes are part of the algorithm and ignored.
        // Note that an overflow would require 2^56 bytes pushed into the
        // engine, which would take thousands of years to compute with the
        // implemented algorithm.
        let total: u64 = self.total.checked_mul(8).unwrap();

        // Stream buffer is immediately committed when full, so there
        // must always be remaining space. Push a final 1-bit and pad with
        // 0-bits until the next byte boundary.
        assert_ne!(self.remaining, 0);
        self.chunk[self.chunk.len() - self.remaining] = 0x80u8;
        self.remaining = self.remaining - 1;

        // Now pad with 0-bytes until the chunk is full, except for 8 final
        // bytes. Those take the total size in bits. If it does not fit into
        // the current chunk, start a new one.
        if self.remaining < 8 {
            self.push_zero(self.remaining);
        }
        let pad = self.remaining.checked_sub(8).unwrap();
        self.push_zero(pad);
        self.push(&total.to_be_bytes());

        // Ensure that we committed the chunk.
        assert_eq!(self.remaining, self.chunk.len());

        // Turn the final hash value into bytes.
        let fn_flatten = |v: [[u8; 4]; 8]| {[
            v[0][0], v[0][1], v[0][2], v[0][3],
            v[1][0], v[1][1], v[1][2], v[1][3],
            v[2][0], v[2][1], v[2][2], v[2][3],
            v[3][0], v[3][1], v[3][2], v[3][3],
            v[4][0], v[4][1], v[4][2], v[4][3],
            v[5][0], v[5][1], v[5][2], v[5][3],
            v[6][0], v[6][1], v[6][2], v[6][3],
            v[7][0], v[7][1], v[7][2], v[7][3],
        ]};
        let r = crate::hash::Hash(
            fn_flatten([
                self.hash[0].to_be_bytes(),
                self.hash[1].to_be_bytes(),
                self.hash[2].to_be_bytes(),
                self.hash[3].to_be_bytes(),
                self.hash[4].to_be_bytes(),
                self.hash[5].to_be_bytes(),
                self.hash[6].to_be_bytes(),
                self.hash[7].to_be_bytes(),
            ])
        );

        // Reset the engine.
        self.hash = SHA256_H;
        self.total = 0;

        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test the SHA-256 engine against a couple of reference vectors.
    #[test]
    fn reference_vectors() {
        assert_eq!(
            crate::hash::hash::<Engine>(
                &[],
            ).to_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        );

        assert_eq!(
            crate::hash::hash::<Engine>(
                b"foobar",
            ).to_hex(),
            "c3ab8ff13720e8ad9047dd39466b3c8974e592c2fa383d4a3960714caef0c4f2"
        );
    }

    // Verify `Engine` is object safe.
    #[test]
    fn object_safety() {
        let e: &mut dyn crate::hash::Engine<Hash = _> = &mut <Engine as Default>::default();

        assert_eq!(
            e.finalize().to_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        );
    }

    // Verify splitting of the input does not affect the output.
    #[test]
    fn split() {
        let e: &mut dyn crate::hash::Engine<Hash = _> = &mut <Engine as Default>::default();

        e.push(b"f");
        e.push(b"o");
        e.push(b"o");
        e.push(b"b");
        e.push(b"a");
        e.push(b"r");

        assert_eq!(
            e.finalize().to_hex(),
            "c3ab8ff13720e8ad9047dd39466b3c8974e592c2fa383d4a3960714caef0c4f2"
        );
    }

    // Run a suite of fixed zero cases.
    #[test]
    fn zeroes() {
        let e: &mut dyn crate::hash::Engine<Hash = _> = &mut <Engine as Default>::default();
        let cases = [
            (  54, "ea659cdc838619b3767c057fdf8e6d99fde2680c5d8517eb06761c0878d40c40"),
            (  55, "02779466cdec163811d078815c633f21901413081449002f24aa3e80f0b88ef7"),
            (  56, "d4817aa5497628e7c77e6b606107042bbba3130888c5f47a375e6179be789fbb"),
            (  57, "65a16cb7861335d5ace3c60718b5052e44660726da4cd13bb745381b235a1785"),
            (  58, "66b4a8b2a17f0463f7427c0239106eaf710ea7129f42d184a58c50cdff614ba4"),
            (  64, "f5a5fd42d16a20302798ef6ed309979b43003d2320d9f0e8ea9831a92759fb4b"),
            ( 128, "38723a2e5e8a17aa7950dc008209944e898f69a7bd10a23c839d341e935fd5ca"),
            ( 256, "5341e6b2646979a70e57653007a1f310169421ec9bdd9f1a5648f75ade005af1"),
            ( 512, "076a27c79e5ace2a3d47f9dd2e83e4ff6ea8872b3c2218f66c92b89b55f36560"),
            (1024, "5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef"),
        ];

        for case in cases.iter() {
            e.push_zero(case.0);
            assert_eq!(
                e.finalize().to_hex(),
                case.1,
            );
        }
    }
}
