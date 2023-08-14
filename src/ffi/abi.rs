//! ABI Abstraction
//!
//! This module provides the `Abi` trait, which is a type-collection used to
//! abstract over the ABI of the target platform. The `Abi` trait can be used
//! to access data of native and foreign ABIs independent of the ABI of the
//! calling platform.

use crate::ffi;
use crate::mem::align;

/// ## ABI Description
///
/// This trait defines properties of a system ABI. It provides associated types
/// for all common data-types used in a given ABI.
pub trait Abi {
    /// ZST with alignment for 8-byte types of the platform.
    type Align1: Copy;
    /// ZST with alignment for 16-byte types of the platform.
    type Align2: Copy;
    /// ZST with alignment for 32-byte types of the platform.
    type Align4: Copy;
    /// ZST with alignment for 64-byte types of the platform.
    type Align8: Copy;
    /// ZST with alignment for 128-byte types of the platform.
    type Align16: Copy;
    /// ZST with native alignment of the platform, used as phantom-type to
    /// raise alignment requirements of a type to the native alignment.
    type AlignNative: Copy;

    /// Native address type for non-NULL values.
    type Addr: Copy;
    /// Native pointer type of the platform, pointing to a value of type
    /// `Target`.
    type Ptr<Target>: Copy;
    /// Native integer type used to encode C-enum values. Note that C-enums
    /// are highly compiler specific and do not necessarily match the type of
    /// the integer constants that make up the enum (yet, they must be able to
    /// represent all values).
    ///
    /// In C, the enumeration type does not necessarily equal the type of the
    /// enumeration members, but must be suitable to represent their values.
    /// Before C23, there was no way to control the type the compiler would
    /// pick for the enum, yet all relevant compilers picked `int`. Hence, you
    /// should likely use the same as default for all enums.
    ///
    /// Note that this is not necessarily the right type for every C-enum in
    /// every interface. Hence, care must be taken to pick a suitable type if
    /// necessary.
    type Enum: Copy;

    /// Big-endian signed x-bit integer of the platform.
    type Ixbe<Native: Copy, Alignment: Copy>: Copy;
    /// Little-endian signed x-bit integer of the platform.
    type Ixle<Native: Copy, Alignment: Copy>: Copy;
    /// Native-endian signed x-bit integer of the platform.
    type Ix<Native: Copy, Alignment: Copy>: Copy;
    /// Big-endian unsigned x-bit integer of the platform.
    type Uxbe<Native: Copy, Alignment: Copy>: Copy;
    /// Little-endian unsigned x-bit integer of the platform.
    type Uxle<Native: Copy, Alignment: Copy>: Copy;
    /// Native-endian unsigned x-bit integer of the platform.
    type Ux<Native: Copy, Alignment: Copy>: Copy;
    /// Big-endian x-bit float of the platform.
    type Fxbe<Native: Copy, Alignment: Copy>: Copy;
    /// Little-endian x-bit float of the platform.
    type Fxle<Native: Copy, Alignment: Copy>: Copy;
    /// Native-endian x-bit float of the platform.
    type Fx<Native: Copy, Alignment: Copy>: Copy;

    /// Big-endian signed 8-bit integer of the platform.
    type I8be: Copy;
    /// Big-endian signed 16-bit integer of the platform.
    type I16be: Copy;
    /// Big-endian signed 32-bit integer of the platform.
    type I32be: Copy;
    /// Big-endian signed 64-bit integer of the platform.
    type I64be: Copy;
    /// Big-endian signed 128-bit integer of the platform.
    type I128be: Copy;

    /// Little-endian signed 8-bit integer of the platform.
    type I8le: Copy;
    /// Little-endian signed 16-bit integer of the platform.
    type I16le: Copy;
    /// Little-endian signed 32-bit integer of the platform.
    type I32le: Copy;
    /// Little-endian signed 64-bit integer of the platform.
    type I64le: Copy;
    /// Little-endian signed 128-bit integer of the platform.
    type I128le: Copy;

    /// Native-endian signed 8-bit integer of the platform.
    type I8: Copy;
    /// Native-endian signed 16-bit integer of the platform.
    type I16: Copy;
    /// Native-endian signed 32-bit integer of the platform.
    type I32: Copy;
    /// Native-endian signed 64-bit integer of the platform.
    type I64: Copy;
    /// Native-endian signed 128-bit integer of the platform.
    type I128: Copy;

    /// Big-endian unsigned 8-bit integer of the platform.
    type U8be: Copy;
    /// Big-endian unsigned 16-bit integer of the platform.
    type U16be: Copy;
    /// Big-endian unsigned 32-bit integer of the platform.
    type U32be: Copy;
    /// Big-endian unsigned 64-bit integer of the platform.
    type U64be: Copy;
    /// Big-endian unsigned 128-bit integer of the platform.
    type U128be: Copy;

    /// Little-endian unsigned 8-bit integer of the platform.
    type U8le: Copy;
    /// Little-endian unsigned 16-bit integer of the platform.
    type U16le: Copy;
    /// Little-endian unsigned 32-bit integer of the platform.
    type U32le: Copy;
    /// Little-endian unsigned 64-bit integer of the platform.
    type U64le: Copy;
    /// Little-endian unsigned 128-bit integer of the platform.
    type U128le: Copy;

    /// Native-endian unsigned 8-bit integer of the platform.
    type U8: Copy;
    /// Native-endian unsigned 16-bit integer of the platform.
    type U16: Copy;
    /// Native-endian unsigned 32-bit integer of the platform.
    type U32: Copy;
    /// Native-endian unsigned 64-bit integer of the platform.
    type U64: Copy;
    /// Native-endian unsigned 128-bit integer of the platform.
    type U128: Copy;

    /// Big-endian 32-bit float of the platform.
    type F32be: Copy;
    /// Big-endian 64-bit float of the platform.
    type F64be: Copy;
    /// Little-endian 32-bit float of the platform.
    type F32le: Copy;
    /// Little-endian 64-bit float of the platform.
    type F64le: Copy;
    /// Native-endian 32-bit float of the platform.
    type F32: Copy;
    /// Native-endian 64-bit float of the platform.
    type F64: Copy;
}

/// ## Native ABI
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Native {}

/// ## Big-endian 32-bit ABI
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Abi32be {}

/// ## Little-endian 32-bit ABI
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Abi32le {}

/// ## Big-endian 64-bit ABI
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Abi64be {}

/// ## Little-endian 64-bit ABI
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Abi64le {}

macro_rules! supplement_abi_common {
    () => {
        type Align1 = align::Align1;
        type Align2 = align::Align2;
        type Align4 = align::Align4;

        // Rely on `target_pointer_width` being 4 or 8.
        type Align8 = Self::AlignNative;
        type Align16 = Self::AlignNative;

        type Enum = Self::I32;
    }
}

// Supplement `Abi` implementations with `Integer` based endian converters.
macro_rules! supplement_abi_integer {
    () => {
        type Ixbe<Native: Copy, Alignment: Copy> = ffi::util::Integer<ffi::util::BigEndian<Native>, Alignment, Native>;
        type Ixle<Native: Copy, Alignment: Copy> = ffi::util::Integer<ffi::util::LittleEndian<Native>, Alignment, Native>;
        type Uxbe<Native: Copy, Alignment: Copy> = ffi::util::Integer<ffi::util::BigEndian<Native>, Alignment, Native>;
        type Uxle<Native: Copy, Alignment: Copy> = ffi::util::Integer<ffi::util::LittleEndian<Native>, Alignment, Native>;
        type Fxbe<Native: Copy, Alignment: Copy> = ffi::util::Integer<ffi::util::BigEndian<Native>, Alignment, Native>;
        type Fxle<Native: Copy, Alignment: Copy> = ffi::util::Integer<ffi::util::LittleEndian<Native>, Alignment, Native>;

        type I8be = Self::Ixbe<i8, Self::Align1>;
        type I16be = Self::Ixbe<i16, Self::Align2>;
        type I32be = Self::Ixbe<i32, Self::Align4>;
        type I64be = Self::Ixbe<i64, Self::Align8>;
        type I128be = Self::Ixbe<i128, Self::Align16>;

        type I8le = Self::Ixle<i8, Self::Align1>;
        type I16le = Self::Ixle<i16, Self::Align2>;
        type I32le = Self::Ixle<i32, Self::Align4>;
        type I64le = Self::Ixle<i64, Self::Align8>;
        type I128le = Self::Ixle<i128, Self::Align16>;

        type U8be = Self::Uxbe<u8, Self::Align1>;
        type U16be = Self::Uxbe<u16, Self::Align2>;
        type U32be = Self::Uxbe<u32, Self::Align4>;
        type U64be = Self::Uxbe<u64, Self::Align8>;
        type U128be = Self::Uxbe<u128, Self::Align16>;

        type U8le = Self::Uxle<u8, Self::Align1>;
        type U16le = Self::Uxle<u16, Self::Align2>;
        type U32le = Self::Uxle<u32, Self::Align4>;
        type U64le = Self::Uxle<u64, Self::Align8>;
        type U128le = Self::Uxle<u128, Self::Align16>;

        type F32be = Self::Fxbe<f32, Self::Align4>;
        type F64be = Self::Fxbe<f64, Self::Align8>;
        type F32le = Self::Fxle<f32, Self::Align4>;
        type F64le = Self::Fxle<f64, Self::Align8>;
    }
}

// Supplement `Abi` implementations with the default target aliases.
macro_rules! supplement_abi_target {
    () => {
        type I8 = Self::Ix<i8, Self::Align1>;
        type I16 = Self::Ix<i16, Self::Align2>;
        type I32 = Self::Ix<i32, Self::Align4>;
        type I64 = Self::Ix<i64, Self::Align8>;
        type I128 = Self::Ix<i128, Self::Align16>;

        type U8 = Self::Ux<u8, Self::Align1>;
        type U16 = Self::Ux<u16, Self::Align2>;
        type U32 = Self::Ux<u32, Self::Align4>;
        type U64 = Self::Ux<u64, Self::Align8>;
        type U128 = Self::Ux<u128, Self::Align16>;

        type F32 = Self::Fx<f32, Self::Align4>;
        type F64 = Self::Fx<f64, Self::Align8>;
    }
}

impl Abi for Native {
    type AlignNative = align::AlignNative;

    supplement_abi_common!();
    supplement_abi_integer!();

    type Addr = usize;
    type Ptr<Target> = core::ptr::NonNull<Target>;

    type Ix<Native: Copy, Alignment: Copy> = Self::Ixbe<Native, Alignment>;
    type Ux<Native: Copy, Alignment: Copy> = Self::Uxbe<Native, Alignment>;
    type Fx<Native: Copy, Alignment: Copy> = Self::Fxbe<Native, Alignment>;

    type I8 = i8;
    type I16 = i16;
    type I32 = i32;
    type I64 = i64;
    type I128 = i128;

    type U8 = u8;
    type U16 = u16;
    type U32 = u32;
    type U64 = u64;
    type U128 = u128;

    type F32 = f32;
    type F64 = f64;
}

impl Abi for Abi32be {
    type AlignNative = align::Align4;

    supplement_abi_common!();
    supplement_abi_integer!();

    type Addr = ffi::util::Integer<ffi::util::BigEndian<core::num::NonZeroU32>, Self::AlignNative, core::num::NonZeroU32>;
    type Ptr<Target> = ffi::util::Pointer<Self::Addr, Target>;

    type Ix<Native: Copy, Alignment: Copy> = Self::Ixbe<Native, Alignment>;
    type Ux<Native: Copy, Alignment: Copy> = Self::Uxbe<Native, Alignment>;
    type Fx<Native: Copy, Alignment: Copy> = Self::Fxbe<Native, Alignment>;

    supplement_abi_target!();
}

impl Abi for Abi32le {
    type AlignNative = align::Align4;

    supplement_abi_common!();
    supplement_abi_integer!();

    type Addr = ffi::util::Integer<ffi::util::LittleEndian<core::num::NonZeroU32>, Self::AlignNative, core::num::NonZeroU32>;
    type Ptr<Target> = ffi::util::Pointer<Self::Addr, Target>;

    type Ix<Native: Copy, Alignment: Copy> = Self::Ixle<Native, Alignment>;
    type Ux<Native: Copy, Alignment: Copy> = Self::Uxle<Native, Alignment>;
    type Fx<Native: Copy, Alignment: Copy> = Self::Fxle<Native, Alignment>;

    supplement_abi_target!();
}

impl Abi for Abi64be {
    type AlignNative = align::Align8;

    supplement_abi_common!();
    supplement_abi_integer!();

    type Addr = ffi::util::Integer<ffi::util::BigEndian<core::num::NonZeroU64>, Self::AlignNative, core::num::NonZeroU64>;
    type Ptr<Target> = ffi::util::Pointer<Self::Addr, Target>;

    type Ix<Native: Copy, Alignment: Copy> = Self::Ixbe<Native, Alignment>;
    type Ux<Native: Copy, Alignment: Copy> = Self::Uxbe<Native, Alignment>;
    type Fx<Native: Copy, Alignment: Copy> = Self::Fxbe<Native, Alignment>;

    supplement_abi_target!();
}

impl Abi for Abi64le {
    type AlignNative = align::Align8;

    supplement_abi_common!();
    supplement_abi_integer!();

    type Addr = ffi::util::Integer<ffi::util::LittleEndian<core::num::NonZeroU64>, Self::AlignNative, core::num::NonZeroU64>;
    type Ptr<Target> = ffi::util::Pointer<Self::Addr, Target>;

    type Ix<Native: Copy, Alignment: Copy> = Self::Ixle<Native, Alignment>;
    type Ux<Native: Copy, Alignment: Copy> = Self::Uxle<Native, Alignment>;
    type Fx<Native: Copy, Alignment: Copy> = Self::Fxle<Native, Alignment>;

    supplement_abi_target!();
}

/// ## Abi Alias for Target Platform
///
/// This is an alias for one of the platform-specific ABI types (e.g.,
/// `Abi64le`, `Abi32be`). This type aliases the type that corresponds to the
/// ABI of the target platform.
///
/// For documentation reasons, it is an alias to `Native`.
#[cfg(doc)]
pub type Target = Native;
#[cfg(all(not(doc), target_endian = "big", target_pointer_width = "32"))]
pub type Target = Abi32be;
#[cfg(all(not(doc), target_endian = "little", target_pointer_width = "32"))]
pub type Target = Abi32le;
#[cfg(all(not(doc), target_endian = "big", target_pointer_width = "64"))]
pub type Target = Abi64be;
#[cfg(all(not(doc), target_endian = "little", target_pointer_width = "64"))]
pub type Target = Abi64le;

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::align_of;

    // Verify typeinfo of basic types
    //
    // Check for size and alignment constaints on all helper types that have a
    // guaranteed layout.
    #[test]
    fn typeinfo() {
        assert_eq!(align_of::<<Abi32be as Abi>::Align1>(), 1);
        assert_eq!(align_of::<<Abi32be as Abi>::Align2>(), 2);
        assert_eq!(align_of::<<Abi32be as Abi>::Align4>(), 4);
        assert_eq!(align_of::<<Abi32be as Abi>::Align8>(), 4);
        assert_eq!(align_of::<<Abi32be as Abi>::Align16>(), 4);
        assert_eq!(align_of::<<Abi32be as Abi>::AlignNative>(), 4);
        assert_eq!(align_of::<<Abi32le as Abi>::Align1>(), 1);
        assert_eq!(align_of::<<Abi32le as Abi>::Align2>(), 2);
        assert_eq!(align_of::<<Abi32le as Abi>::Align4>(), 4);
        assert_eq!(align_of::<<Abi32le as Abi>::Align8>(), 4);
        assert_eq!(align_of::<<Abi32le as Abi>::Align16>(), 4);
        assert_eq!(align_of::<<Abi32le as Abi>::AlignNative>(), 4);
        assert_eq!(align_of::<<Abi64be as Abi>::Align1>(), 1);
        assert_eq!(align_of::<<Abi64be as Abi>::Align2>(), 2);
        assert_eq!(align_of::<<Abi64be as Abi>::Align4>(), 4);
        assert_eq!(align_of::<<Abi64be as Abi>::Align8>(), 8);
        assert_eq!(align_of::<<Abi64be as Abi>::Align16>(), 8);
        assert_eq!(align_of::<<Abi64be as Abi>::AlignNative>(), 8);
        assert_eq!(align_of::<<Abi64le as Abi>::Align1>(), 1);
        assert_eq!(align_of::<<Abi64le as Abi>::Align2>(), 2);
        assert_eq!(align_of::<<Abi64le as Abi>::Align4>(), 4);
        assert_eq!(align_of::<<Abi64le as Abi>::Align8>(), 8);
        assert_eq!(align_of::<<Abi64le as Abi>::Align16>(), 8);
        assert_eq!(align_of::<<Abi64le as Abi>::AlignNative>(), 8);
    }
}
