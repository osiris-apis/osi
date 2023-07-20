//! # Utility Module
//!
//! This is a utility module for the other `ffi` modules. It provides common
//! abstractions and type definitions used across many different interfaces.

/// ## Enumeration Content
///
/// When interfaces use C-enum types, we use this as backing type for all
/// its integer constants by default. Note that C-enums are highly compiler
/// specific and do not necessarily match the type of the integer constants
/// that make up the enum (yet, they must be able to represent all values).
///
/// In C, the enumeration type does not necessarily equal the type of the
/// enumeration members, but must be suitable to represent their values.
/// Before C23, there was no way to control the type the compiler would
/// pick for the enum, yet all relevant compilers picked `int`. Hence, we
/// use the same as default for all enums.
///
/// Note that this is not necessarily the right type for every C-enum in
/// every interface. Hence, care must be taken to pick a suitable type if
/// necessary.
pub type Enumeration = i32;

/// ## Anonymous Pointer Content
///
/// When interfaces (e.g., JNI) use anonymous pointer targets, we declare
/// them as new-type structs to a tuple with this type. That is, this type
/// controls the content-alignment and content-size of the pointer type. In
/// most cases, this is irrelevant. However, when dealing with
/// pointer-tagging, it will get relevant, since it defines the used bits
/// of the pointer.
///
/// We use `u8` as content type by default, to prevent accidentally
/// allowing bits to be recycled for other use. Use a different type if the
/// given interface specifies a required target type alignment.
pub type Anonymous = u8;

/// ## An 8-bit aligned ZST
///
/// This type can be used to align structures to at least 8-bit by
/// embedding it in the structure. It works similar to other phantom-marker
/// types.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C, align(1))]
pub struct PhantomAlign8 {}

/// ## A 16-bit aligned ZST
///
/// This type can be used to align structures to at least 16-bit by
/// embedding it in the structure. It works similar to other phantom-marker
/// types.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C, align(2))]
pub struct PhantomAlign16 {}

/// ## A 32-bit aligned ZST
///
/// This type can be used to align structures to at least 32-bit by
/// embedding it in the structure. It works similar to other phantom-marker
/// types.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C, align(4))]
pub struct PhantomAlign32 {}

/// ## A 64-bit aligned ZST
///
/// This type can be used to align structures to at least 64-bit by
/// embedding it in the structure. It works similar to other phantom-marker
/// types.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C, align(8))]
pub struct PhantomAlign64 {}

/// ## Value Selector based on Address Size
///
/// Return either of the arguments, depending on the pointer-width of the
/// compilation target. For 32-bit machines `v32` is returned, for 64-bit
/// machines `v64` is returned.
#[allow(unused)]
pub const fn v32_v64<T>(v32: T, v64: T) -> T
where
    T: Copy,
{
    let v: T;
    #[cfg(target_pointer_width = "32")]
    { v = v32; }
    #[cfg(target_pointer_width = "64")]
    { v = v64; }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    // Verify typeinfo of basic types
    //
    // Check for size and alignment constaints on all helper types that have a
    // guaranteed layout.
    #[test]
    fn typeinfo() {
        assert_eq!(size_of::<Enumeration>(), 4);
        assert_eq!(align_of::<Enumeration>(), 4);

        assert_eq!(size_of::<Anonymous>(), 1);
        assert_eq!(align_of::<Anonymous>(), 1);

        assert_eq!(align_of::<PhantomAlign8>(), 1);
        assert_eq!(align_of::<PhantomAlign16>(), 2);
        assert_eq!(align_of::<PhantomAlign32>(), 4);
        assert_eq!(align_of::<PhantomAlign64>(), 8);
        assert_eq!(size_of::<PhantomAlign8>(), 0);
        assert_eq!(size_of::<PhantomAlign16>(), 0);
        assert_eq!(size_of::<PhantomAlign32>(), 0);
        assert_eq!(size_of::<PhantomAlign64>(), 0);
    }

    // Verify `v32_v64()` selects correctly
    //
    // The `v32_v64()` selector allows encoding pointer-width dependent values
    // at compile-time. Ensure that it selects the right one depending on the
    // native pointer-width.
    #[test]
    fn v32_v64_native() {
        assert_eq!(v32_v64(4, 8), size_of::<usize>());
        assert_eq!(v32_v64(0, 0), 0);
    }
}
