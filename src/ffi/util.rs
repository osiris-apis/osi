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

/// ## A 128-bit aligned ZST
///
/// This type can be used to align structures to at least 128-bit by
/// embedding it in the structure. It works similar to other phantom-marker
/// types.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C, align(16))]
pub struct PhantomAlign128 {}

/// ## Fixed-size non-NULL Pointers
///
/// This type is designed as alternative to `core::ptr::NonNull` but
/// provides a fixed-size address type. It allows representing 32-bit
/// pointers on 64-bit machines, and vice-versa.
///
/// See `Ptr32`, `Ptr64`, and `PtrN` for common type aliases using non-zero
/// integers as address type.
///
/// This type ensures suitable alignment of pointer values independent of the
/// natural alignment of the target platform. That is, 64-bit pointers will
/// always be 8-byte aligned, even on 32-bit machines.
///
/// Wrapping this type in an `Option<...>` is guaranteed to yield a type of
/// the same layout.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Ptr<ADDR, ALIGN, TARGET>
where
    ADDR: Clone + Copy + core::fmt::Debug + Eq + core::hash::Hash + Ord + PartialEq + PartialOrd,
    ALIGN: Clone + Copy + core::fmt::Debug + Eq + core::hash::Hash + Ord + PartialEq + PartialOrd,
    TARGET: ?Sized,
{
    addr: ADDR,
    align: [ALIGN; 0],
    target: core::marker::PhantomData<*const TARGET>,
}

/// ## 32-bit Pointer Alias
///
/// A simple alias for `Ptr` using `NonZeroU32` as backing type, thus ensuring
/// that all addresses of this pointer type are 32-bit in size.
///
/// This type has a fixed alignment and size of 4 on all platforms.
pub type Ptr32<TARGET> = Ptr<core::num::NonZeroU32, PhantomAlign32, TARGET>;

/// ## 64-bit Pointer Alias
///
/// A simple alias for `Ptr` using `NonZeroU64` as backing type, thus ensuring
/// that all addresses of this pointer type are 64-bit in size.
///
/// This type has a fixed alignment and size of 8 on all platforms.
pub type Ptr64<TARGET> = Ptr<core::num::NonZeroU64, PhantomAlign64, TARGET>;

/// ## 128-bit Pointer Alias
///
/// A simple alias for `Ptr` using `NonZeroU128` as backing type, thus ensuring
/// that all addresses of this pointer type are 128-bit in size.
///
/// This type has a fixed alignment and size of 16 on all platforms.
pub type Ptr128<TARGET> = Ptr<core::num::NonZeroU128, PhantomAlign128, TARGET>;

#[cfg(doc)]
/// ## Native Pointer Alias
///
/// This is an alias to either `Ptr32` or `Ptr64` depending on the native
/// pointer width of the target architecture.
pub type PtrN<TARGET> = Ptr64<TARGET>;
#[cfg(all(not(doc), target_pointer_width = "32"))]
pub type PtrN<TARGET> = Ptr32<TARGET>;
#[cfg(all(not(doc), target_pointer_width = "64"))]
pub type PtrN<TARGET> = Ptr64<TARGET>;

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

impl<ADDR, ALIGN, TARGET> Ptr<ADDR, ALIGN, TARGET>
where
    ADDR: Clone + Copy + core::fmt::Debug + Eq + core::hash::Hash + Ord + PartialEq + PartialOrd,
    ALIGN: Clone + Copy + core::fmt::Debug + Eq + core::hash::Hash + Ord + PartialEq + PartialOrd,
    TARGET: ?Sized,
{
    /// ## Create new instance
    ///
    /// Create a new instance of this pointer type from the provided address.
    /// The address is taken verbatim.
    #[inline]
    pub const fn new(v: ADDR) -> Self {
        Self {
            addr: v,
            align: [],
            target: core::marker::PhantomData {},
        }
    }

    /// ## Yield underlying address
    ///
    /// Return the address underlying this pointer type.
    #[inline(always)]
    #[must_use]
    pub const fn addr(self) -> ADDR {
        self.addr
    }

    /// ## Cast pointer
    ///
    /// Change the target pointer type to the specified type. This does not
    /// change the underlying address value.
    #[inline]
    pub const fn cast<OTHER>(self) -> Ptr<ADDR, ALIGN, OTHER> {
        Ptr::<ADDR, ALIGN, OTHER>::new(self.addr())
    }
}

// Implement natural conversion from address to pointer.
impl<ADDR, ALIGN, TARGET> From<ADDR> for Ptr<ADDR, ALIGN, TARGET>
where
    ADDR: Clone + Copy + core::fmt::Debug + Eq + core::hash::Hash + Ord + PartialEq + PartialOrd,
    ALIGN: Clone + Copy + core::fmt::Debug + Eq + core::hash::Hash + Ord + PartialEq + PartialOrd,
    TARGET: ?Sized,
{
    #[inline]
    fn from(v: ADDR) -> Self {
        Self::new(v)
    }
}

// Implement `Ptr` for address-types like `core::num::NonZeroU*`. This will
// provide suitable helpers to convert to and from primitive integers without
// going through the intermediate address-type.
macro_rules! implement_ptr_nonzero {
    ($addr:ty, $align:ty, $raw:ty) => {
        impl<TARGET: ?Sized> Ptr<$addr, $align, TARGET> {
            /// ## Create new instance from raw address
            ///
            /// Create a new instance of this pointer type from the raw,
            /// unchecked address.
            ///
            /// Safety
            /// ------
            ///
            /// The caller must ensure that the raw address is not 0.
            #[inline]
            pub const unsafe fn from_raw_unchecked(v: $raw) -> Self {
                // SAFETY: Delegated to the caller.
                unsafe {
                    Self::new(<$addr>::new_unchecked(v))
                }
            }

            /// ## Create new instance from raw address
            ///
            /// Create a new instance of this pointer type from the raw
            /// address, yielding `None` if it is 0.
            #[inline]
            pub const fn from_raw(v: $raw) -> Option<Self> {
                if let Some(addr) = <$addr>::new(v) {
                    Some(Self::new(addr))
                } else {
                    None
                }
            }

            /// ## Yield raw address
            ///
            /// Return the raw address underlying this pointer type. This raw
            /// address can never be 0.
            #[inline(always)]
            #[must_use]
            pub const fn raw(self) -> $raw {
                self.addr.get()
            }
        }

        // Implement natural conversion from raw address to pointer.
        impl<TARGET: ?Sized> TryFrom<$raw> for Ptr<$addr, $align, TARGET> {
            type Error = ();

            #[inline]
            fn try_from(v: $raw) -> Result<Self, Self::Error> {
                Self::from_raw(v).ok_or(())
            }
        }
    }
}

// Supplement `implement_ptr_*()` with converters to and from native pointers,
// assuming the address-type is equal to the native pointer width.
macro_rules! supplement_ptr_native {
    ($addr:ty, $align:ty, $raw:ty) => {
        impl<TARGET: ?Sized> Ptr<$addr, $align, TARGET> {
            // Helper to convert to `usize`, ensuring non-fallible cast.
            #[inline(always)]
            const fn raw_to_usize(v: $raw) -> usize {
                assert!(core::mem::size_of::<$raw>() >= core::mem::size_of::<usize>());
                v as usize
            }

            // Helper to convert from `usize`, ensuring non-fallible cast.
            #[inline(always)]
            const fn usize_to_raw(v: usize) -> $raw {
                assert!(core::mem::size_of::<usize>() >= core::mem::size_of::<$raw>());
                v as $raw
            }

            /// ## Create new instance from `usize`
            ///
            /// Create a new instance of this pointer type with the address
            /// specified as a `usize` value.
            ///
            /// Safety
            /// ------
            ///
            /// The caller must ensure that the address is not 0.
            #[inline]
            pub const unsafe fn from_usize_unchecked(v: usize) -> Self {
                Self::from_raw_unchecked(Self::usize_to_raw(v))
            }

            /// ## Create new instance from `usize`
            ///
            /// Create a new instance of this pointer type with the address
            /// specified as a `usize` value. If the address is 0, this
            /// will yield `None`.
            #[inline]
            pub const fn from_usize(v: usize) -> Option<Self> {
                Self::from_raw(Self::usize_to_raw(v))
            }

            /// ## Yield address as `usize`
            ///
            /// Return the address underlying this pointer as a `usize`.
            #[inline(always)]
            #[must_use]
            pub const fn as_usize(self) -> usize {
                Self::raw_to_usize(self.raw())
            }
        }

        impl<TARGET: ?Sized> TryFrom<usize> for Ptr<$addr, $align, TARGET> {
            type Error = ();

            #[inline]
            fn try_from(v: usize) -> Result<Self, Self::Error> {
                Self::from_usize(v).ok_or(())
            }
        }

        impl<TARGET: Sized> Ptr<$addr, $align, TARGET> {
            /// ## Create new dangling pointer
            ///
            /// Create a new instance of this pointer with a dangling address.
            /// This address is guaranteed to not be 0. However, the address is
            /// not necessarily unique and might match a valid address of
            /// another allocated object.
            #[inline]
            #[must_use]
            pub const fn dangling() -> Self {
                // SAFETY: Alignments cannot be 0.
                unsafe {
                    Self::from_usize_unchecked(
                        core::mem::align_of::<TARGET>(),
                    )
                }
            }

            /// ## Yield address as raw pointer
            ///
            /// Return the address underlying this pointer as a raw pointer
            /// type. This pointer is guaranteed to not be NULL.
            #[inline(always)]
            #[must_use]
            pub const fn as_ptr(self) -> *const TARGET {
                self.as_usize() as *const TARGET
            }

            /// ## Yield address as raw mutable pointer
            ///
            /// Return the address underlying this pointer as a raw pointer
            /// pointer type. This pointer is guaranteed to not be NULL.
            #[inline(always)]
            #[must_use]
            pub const fn as_mut_ptr(self) -> *mut TARGET {
                self.as_usize() as *mut TARGET
            }

            /// ## Yield address as reference
            ///
            /// Return the address underlying this pointer as a reference to
            /// the target type.
            ///
            /// Safety
            /// ------
            ///
            /// The caller must ensure that the underlying address can be
            /// safely cast into a reference, following the usual requirements
            /// of the Rust language.
            #[inline(always)]
            #[must_use]
            pub const unsafe fn as_ref<'a>(self) -> &'a TARGET {
                // SAFETY: Delegated to caller.
                unsafe { &*self.as_ptr() }
            }

            /// ## Yield address as mutable reference
            ///
            /// Return the address underlying this pointer as a mutable
            /// reference to the target type.
            ///
            /// Safety
            /// ------
            ///
            /// The caller must ensure that the underlying address can be
            /// safely cast into a mutable reference, following the usual
            /// requirements of the Rust language.
            #[inline(always)]
            #[must_use]
            pub unsafe fn as_mut<'a>(self) -> &'a mut TARGET {
                // SAFETY: Delegated to caller.
                unsafe { &mut *(self.as_ptr() as *mut TARGET) }
            }
        }

        impl<TARGET: Sized> From<&TARGET> for Ptr<$addr, $align, TARGET> {
            #[inline]
            fn from(v: &TARGET) -> Self {
                // SAFETY: References cannot be NULL.
                unsafe {
                    Self::from_raw_unchecked(
                        v as *const TARGET as usize as $raw,
                    )
                }
            }
        }

        impl<TARGET: Sized> From<&mut TARGET> for Ptr<$addr, $align, TARGET> {
            #[inline]
            fn from(v: &mut TARGET) -> Self {
                // SAFETY: References cannot be NULL.
                unsafe {
                    Self::from_raw_unchecked(
                        v as *mut TARGET as usize as $raw,
                    )
                }
            }
        }

        impl<TARGET: Sized> TryFrom<*const TARGET> for Ptr<$addr, $align, TARGET> {
            type Error = ();

            #[inline]
            fn try_from(v: *const TARGET) -> Result<Self, Self::Error> {
                Self::from_raw(v as usize as $raw).ok_or(())
            }
        }

        impl<TARGET: Sized> TryFrom<*mut TARGET> for Ptr<$addr, $align, TARGET> {
            type Error = ();

            #[inline]
            fn try_from(v: *mut TARGET) -> Result<Self, Self::Error> {
                Self::from_raw(v as usize as $raw).ok_or(())
            }
        }
    }
}

// Implement `Ptr<NonZeroU*>` for common pointer sizes.
implement_ptr_nonzero!(core::num::NonZeroU32, PhantomAlign32, u32);
implement_ptr_nonzero!(core::num::NonZeroU64, PhantomAlign64, u64);
implement_ptr_nonzero!(core::num::NonZeroU128, PhantomAlign128, u128);

// Supplement the native pointer type with converters to raw pointers et al.
#[cfg(target_pointer_width = "32")]
supplement_ptr_native!(core::num::NonZeroU32, PhantomAlign32, u32);
#[cfg(target_pointer_width = "64")]
supplement_ptr_native!(core::num::NonZeroU64, PhantomAlign64, u64);

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
        assert_eq!(align_of::<PhantomAlign128>(), 16);
        assert_eq!(size_of::<PhantomAlign8>(), 0);
        assert_eq!(size_of::<PhantomAlign16>(), 0);
        assert_eq!(size_of::<PhantomAlign32>(), 0);
        assert_eq!(size_of::<PhantomAlign64>(), 0);
        assert_eq!(size_of::<PhantomAlign128>(), 0);

        assert_eq!(size_of::<Ptr32<()>>(), 4);
        assert_eq!(align_of::<Ptr32<()>>(), 4);
        assert_eq!(size_of::<Ptr64<()>>(), 8);
        assert_eq!(align_of::<Ptr64<()>>(), 8);
        assert_eq!(size_of::<Ptr128<()>>(), 16);
        assert_eq!(align_of::<Ptr128<()>>(), 16);
        assert_eq!(size_of::<PtrN<()>>(), v32_v64(4, 8));
        assert_eq!(align_of::<PtrN<()>>(), v32_v64(4, 8));

        assert_eq!(size_of::<Option<Ptr32<()>>>(), 4);
        assert_eq!(align_of::<Option<Ptr32<()>>>(), 4);
        assert_eq!(size_of::<Option<Ptr64<()>>>(), 8);
        assert_eq!(align_of::<Option<Ptr64<()>>>(), 8);
        assert_eq!(size_of::<Option<Ptr128<()>>>(), 16);
        assert_eq!(align_of::<Option<Ptr128<()>>>(), 16);
        assert_eq!(size_of::<Option<PtrN<()>>>(), v32_v64(4, 8));
        assert_eq!(align_of::<Option<PtrN<()>>>(), v32_v64(4, 8));
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
