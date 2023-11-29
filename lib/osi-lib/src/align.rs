//! # Type Alignment
//!
//! This module provides utilities to manage and query alignment of data-types
//! and memory in Rust.
//!
//! ## Alignment Phantom Marker
//!
//! The data-types named `AlignXyz` are ZSTs, modeled after other phantom
//! markers of the rust standard library. They can be embedded in other
//! data-structures and ensure to raise their alignment to the alignment of
//! `AlignXyz`.
//!
//! The fixed-size markers `Align1`, `Align2`, ..., `Align128` raise the
//! alignment to a fixed value. The `AlignAs` type uses a const-generic to
//! allow specifying the alignment as a compile-time constant. The `AlignOf`
//! type allows propagating the alignment of another type.
//!
//! Note that these markers only work for types with alignments up to `128`
//! bytes.
//!
//! ## Nameing
//!
//! While data-types use their bit-size as name (e.g., `u8`, `u32`), all
//! references to alignment always use the byte-size (e.g., `repr(align(...))`
//! uses bytes, not bits). Furthermore, runtime functions return values in
//! bytes (e.g., `core::mem::align_of()` returns bytes, not bits).
//!
//! Furthermore, const-generics naturally specify size and alignment in bytes
//! to better integrate with `core::mem`.
//!
//! Therefore, to produce predictable names, all alignment-types use byte-based
//! alignment suffixes, rather than bit-based. In fact, as a rule of thumb, all
//! references to size and alignment are specified in bytes, **except** for the
//! names of builtin primitive integer types, which use bits for historic
//! reasons.

use crate::dd;

/// ## A 1-byte (8-bit) aligned ZST
///
/// This type can be used to align structures to at least 1-byte by
/// embedding it in the structure. It works similar to other phantom-marker
/// types.
///
/// Note that all data-types are always aligned to at least 1-byte, hence this
/// marker has no effect on type layout when embedded in other types.
#[derive(Clone, Copy, Debug, Default, Hash)]
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[repr(C, align(1))]
pub struct Align1 {}

/// ## A 2-byte (16-bit) aligned ZST
///
/// This type can be used to align structures to at least 2-bytes by
/// embedding it in the structure. It works similar to other phantom-marker
/// types.
#[derive(Clone, Copy, Debug, Default, Hash)]
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[repr(C, align(2))]
pub struct Align2 {}

/// ## A 4-byte (32-bit) aligned ZST
///
/// This type can be used to align structures to at least 4-bytes by
/// embedding it in the structure. It works similar to other phantom-marker
/// types.
#[derive(Clone, Copy, Debug, Default, Hash)]
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[repr(C, align(4))]
pub struct Align4 {}

/// ## An 8-byte (64-bit) aligned ZST
///
/// This type can be used to align structures to at least 8-bytes by
/// embedding it in the structure. It works similar to other phantom-marker
/// types.
#[derive(Clone, Copy, Debug, Default, Hash)]
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[repr(C, align(8))]
pub struct Align8 {}

/// ## A 16-byte (128-bit) aligned ZST
///
/// This type can be used to align structures to at least 16-bytes by
/// embedding it in the structure. It works similar to other phantom-marker
/// types.
#[derive(Clone, Copy, Debug, Default, Hash)]
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[repr(C, align(16))]
pub struct Align16 {}

/// ## A 32-byte (256-bit) aligned ZST
///
/// This type can be used to align structures to at least 32-bytes by
/// embedding it in the structure. It works similar to other phantom-marker
/// types.
#[derive(Clone, Copy, Debug, Default, Hash)]
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[repr(C, align(32))]
pub struct Align32 {}

/// ## A 64-byte (512-bit) aligned ZST
///
/// This type can be used to align structures to at least 64-bytes by
/// embedding it in the structure. It works similar to other phantom-marker
/// types.
#[derive(Clone, Copy, Debug, Default, Hash)]
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[repr(C, align(64))]
pub struct Align64 {}

/// ## A 128-byte (1024-bit) aligned ZST
///
/// This type can be used to align structures to at least 128-bytes by
/// embedding it in the structure. It works similar to other phantom-marker
/// types.
#[derive(Clone, Copy, Debug, Default, Hash)]
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[repr(C, align(128))]
pub struct Align128 {}

/// ## Alignment Marker Trait
///
/// This trait is a marker trait to describe the alignment of a data type. If
/// implemented for a type, the associated type `Align` must be set to one of
/// the phantom-markers of this module, which represents the alignment of the
/// data-type.
///
/// That is, for a 4-byte aligned type, `Aligned::Align` must be set to
/// `Align4`.
///
/// In most cases, `Aligned::Align` can be set to
/// `AlignAs<{core::mem::align_of::<Self>()}>`. However, due to lack of
/// compiler support for generic types in const-generics, this cannot be
/// provided as a blanket implementation.
///
/// ### Safety
///
/// An implementation must guarantee that `Aligned::Align` is a ZST and has the
/// same alignment requirements as `Self`.
pub unsafe trait Aligned {
    type Align: Sized;
}

/// ## A ZST with fixed alignment
///
/// This type can be used to align structures to at least the alignment given
/// as `BYTES` by embedding it in the structure. It works similar to other
/// phantom-marker types.
#[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default, dd::Hash)]
#[dd::derive(dd::Eq, dd::Ord, dd::PartialEq, dd::PartialOrd)]
#[repr(transparent)]
pub struct AlignAs<const BYTES: usize>(
    <Self as Aligned>::Align,
) where Self: Aligned;

/// ## A ZST with propagated alignment
///
/// This type can be used to align structures to at least the alignment of the
/// type `Of` by embedding it in the structure. It works similar to other
/// phantom-marker types.
#[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default, dd::Hash)]
#[dd::derive(dd::Eq, dd::Ord, dd::PartialEq, dd::PartialOrd)]
#[repr(transparent)]
pub struct AlignOf<Of>(
    Of::Align,
) where Of: Aligned + ?Sized;

/// ## An unaligned ZST
///
/// This type is a ZST with 1-byte alignment, and thus has no effect on type
/// layout when embedded in other types. It exists as fallback to be passed
/// as alignment-restriction, when no such restriction is intended. It is a
/// simple alias for `Align1`, since it has the same effect.
pub type AlignAny = Align1;

/// ## A native-aligned ZST
///
/// This type can be used to align structures to at least the native alignment
/// by embedding it in the structure. It works similar to other phantom-marker
/// types.
pub type AlignNative = AlignOf<usize>;

unsafe impl Aligned for AlignAs<1> { type Align = Align1; }
unsafe impl Aligned for AlignAs<2> { type Align = Align2; }
unsafe impl Aligned for AlignAs<4> { type Align = Align4; }
unsafe impl Aligned for AlignAs<8> { type Align = Align8; }
unsafe impl Aligned for AlignAs<16> { type Align = Align16; }
unsafe impl Aligned for AlignAs<32> { type Align = Align32; }
unsafe impl Aligned for AlignAs<64> { type Align = Align64; }
unsafe impl Aligned for AlignAs<128> { type Align = Align128; }

unsafe impl Aligned for i8 { type Align = AlignAs<{core::mem::align_of::<i8>()}>; }
unsafe impl Aligned for i16 { type Align = AlignAs<{core::mem::align_of::<i16>()}>; }
unsafe impl Aligned for i32 { type Align = AlignAs<{core::mem::align_of::<i32>()}>; }
unsafe impl Aligned for i64 { type Align = AlignAs<{core::mem::align_of::<i64>()}>; }
unsafe impl Aligned for i128 { type Align = AlignAs<{core::mem::align_of::<i128>()}>; }
unsafe impl Aligned for isize { type Align = AlignAs<{core::mem::align_of::<isize>()}>; }
unsafe impl Aligned for u8 { type Align = AlignAs<{core::mem::align_of::<u8>()}>; }
unsafe impl Aligned for u16 { type Align = AlignAs<{core::mem::align_of::<u16>()}>; }
unsafe impl Aligned for u32 { type Align = AlignAs<{core::mem::align_of::<u32>()}>; }
unsafe impl Aligned for u64 { type Align = AlignAs<{core::mem::align_of::<u64>()}>; }
unsafe impl Aligned for u128 { type Align = AlignAs<{core::mem::align_of::<u128>()}>; }
unsafe impl Aligned for usize { type Align = AlignAs<{core::mem::align_of::<usize>()}>; }
unsafe impl Aligned for f32 { type Align = AlignAs<{core::mem::align_of::<f32>()}>; }
unsafe impl Aligned for f64 { type Align = AlignAs<{core::mem::align_of::<f64>()}>; }
unsafe impl Aligned for char { type Align = AlignAs<{core::mem::align_of::<char>()}>; }
unsafe impl Aligned for bool { type Align = AlignAs<{core::mem::align_of::<bool>()}>; }
unsafe impl Aligned for () { type Align = AlignAs<{core::mem::align_of::<()>()}>; }
unsafe impl Aligned for str { type Align = AlignAs<{core::mem::align_of::<u8>()}>; }

unsafe impl<const N: usize, T: Aligned> Aligned for [T; N] { type Align = T::Align; }
unsafe impl<T: Aligned> Aligned for [T] { type Align = T::Align; }
unsafe impl<T> Aligned for &T { type Align = AlignNative; }
unsafe impl<T> Aligned for &mut T { type Align = AlignNative; }
unsafe impl<T> Aligned for *const T { type Align = AlignNative; }
unsafe impl<T> Aligned for *mut T { type Align = AlignNative; }

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use super::*;

    #[test]
    fn typeinfo_alignx() {
        assert_eq!(align_of::<Align1>(), 1);
        assert_eq!(align_of::<Align2>(), 2);
        assert_eq!(align_of::<Align4>(), 4);
        assert_eq!(align_of::<Align8>(), 8);
        assert_eq!(align_of::<Align16>(), 16);
        assert_eq!(align_of::<Align32>(), 32);
        assert_eq!(align_of::<Align64>(), 64);
        assert_eq!(align_of::<Align128>(), 128);
        assert_eq!(align_of::<AlignAny>(), 1);
        assert_eq!(align_of::<AlignNative>(), align_of::<usize>());
        assert_eq!(size_of::<Align1>(), 0);
        assert_eq!(size_of::<Align2>(), 0);
        assert_eq!(size_of::<Align4>(), 0);
        assert_eq!(size_of::<Align8>(), 0);
        assert_eq!(size_of::<Align16>(), 0);
        assert_eq!(size_of::<Align32>(), 0);
        assert_eq!(size_of::<Align64>(), 0);
        assert_eq!(size_of::<Align128>(), 0);
        assert_eq!(size_of::<AlignAny>(), 0);
        assert_eq!(size_of::<AlignNative>(), 0);
    }

    #[test]
    fn typeinfo_align_as() {
        assert_eq!(align_of::<AlignAs<1>>(), 1);
        assert_eq!(align_of::<AlignAs<2>>(), 2);
        assert_eq!(align_of::<AlignAs<4>>(), 4);
        assert_eq!(align_of::<AlignAs<8>>(), 8);
        assert_eq!(align_of::<AlignAs<16>>(), 16);
        assert_eq!(align_of::<AlignAs<32>>(), 32);
        assert_eq!(align_of::<AlignAs<64>>(), 64);
        assert_eq!(align_of::<AlignAs<128>>(), 128);
        assert_eq!(size_of::<AlignAs<1>>(), 0);
        assert_eq!(size_of::<AlignAs<2>>(), 0);
        assert_eq!(size_of::<AlignAs<4>>(), 0);
        assert_eq!(size_of::<AlignAs<8>>(), 0);
        assert_eq!(size_of::<AlignAs<16>>(), 0);
        assert_eq!(size_of::<AlignAs<32>>(), 0);
        assert_eq!(size_of::<AlignAs<64>>(), 0);
        assert_eq!(size_of::<AlignAs<128>>(), 0);
    }

    #[test]
    fn typeinfo_align_of() {
        assert_eq!(align_of::<AlignOf<i8>>(), align_of::<i8>());
        assert_eq!(align_of::<AlignOf<i16>>(), align_of::<i16>());
        assert_eq!(align_of::<AlignOf<i32>>(), align_of::<i32>());
        assert_eq!(align_of::<AlignOf<i64>>(), align_of::<i64>());
        assert_eq!(align_of::<AlignOf<i128>>(), align_of::<i128>());
        assert_eq!(align_of::<AlignOf<isize>>(), align_of::<isize>());
        assert_eq!(align_of::<AlignOf<u8>>(), align_of::<u8>());
        assert_eq!(align_of::<AlignOf<u16>>(), align_of::<u16>());
        assert_eq!(align_of::<AlignOf<u32>>(), align_of::<u32>());
        assert_eq!(align_of::<AlignOf<u64>>(), align_of::<u64>());
        assert_eq!(align_of::<AlignOf<u128>>(), align_of::<u128>());
        assert_eq!(align_of::<AlignOf<usize>>(), align_of::<usize>());
        assert_eq!(align_of::<AlignOf<f32>>(), align_of::<f32>());
        assert_eq!(align_of::<AlignOf<f64>>(), align_of::<f64>());
        assert_eq!(align_of::<AlignOf<char>>(), align_of::<char>());
        assert_eq!(align_of::<AlignOf<bool>>(), align_of::<bool>());
        assert_eq!(align_of::<AlignOf<()>>(), align_of::<()>());
        assert_eq!(align_of::<AlignOf<str>>(), align_of::<u8>());
        assert_eq!(size_of::<AlignOf<i8>>(), 0);
        assert_eq!(size_of::<AlignOf<i16>>(), 0);
        assert_eq!(size_of::<AlignOf<i32>>(), 0);
        assert_eq!(size_of::<AlignOf<i64>>(), 0);
        assert_eq!(size_of::<AlignOf<i128>>(), 0);
        assert_eq!(size_of::<AlignOf<isize>>(), 0);
        assert_eq!(size_of::<AlignOf<u8>>(), 0);
        assert_eq!(size_of::<AlignOf<u16>>(), 0);
        assert_eq!(size_of::<AlignOf<u32>>(), 0);
        assert_eq!(size_of::<AlignOf<u64>>(), 0);
        assert_eq!(size_of::<AlignOf<u128>>(), 0);
        assert_eq!(size_of::<AlignOf<usize>>(), 0);
        assert_eq!(size_of::<AlignOf<f32>>(), 0);
        assert_eq!(size_of::<AlignOf<f64>>(), 0);
        assert_eq!(size_of::<AlignOf<char>>(), 0);
        assert_eq!(size_of::<AlignOf<bool>>(), 0);
        assert_eq!(size_of::<AlignOf<()>>(), 0);
        assert_eq!(size_of::<AlignOf<str>>(), 0);
    }
}
