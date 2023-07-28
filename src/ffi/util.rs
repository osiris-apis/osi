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

/// ## Types of Fixed Endianness
///
/// This trait annotates types that have values of fixed endianness regardless
/// of the target platform endianness. It allows converting values of such
/// types to the native representation, and vice-versa. If the endianness
/// happens to match the endianness of the target platform, all accessors will
/// pass values through unmodified.
///
/// The trait-generic `Raw` defines the type of the native representation. It
/// must be suitable to represent native **and** foreign values. Furthermore,
/// the trait is designed for `Copy` types (in particular primitive integers).
/// Bigger or more complex types are not suitable.
///
/// Safety
/// ------
///
/// This trait requires the implementation to guarantee its size and alignment
/// match that of `Raw`, and it must support transmuting from `Raw`. This
/// allows users to create values of this type by simply transmuting a value of
/// type `Raw`. Since `Raw` represents both foreign and native values, special
/// care is required if the memory representation of `Raw` contains padding or
/// other unaccounted bits!
pub unsafe trait FixedEndian<Raw>
where
    Self: Copy,
    Raw: Copy,
{
    /// Create from raw value
    ///
    /// Take the raw, possibly foreign-ordered value `raw` and create a
    /// wrapping object that protects the value from unguarded access. This
    /// must not modify the raw value in any way.
    ///
    /// It is safe to transmute from `Raw` to `Self` instead.
    fn from_raw(raw: Raw) -> Self;

    /// Return raw value
    ///
    /// Return the underlying raw, possibly foreign-ordered value behind this
    /// wrapping object. The value must be returned without any modifications.
    ///
    /// It is safe to transmute from `Self` to `Raw` instead.
    fn to_raw(self) -> Raw;

    /// Create value from native representation
    ///
    /// Create the foreign-ordered value from a native value, converting the
    /// value before retaining it, if required.
    fn from_native(native: Raw) -> Self;

    /// Return native representation
    ///
    /// Return the native representation of the value behind this wrapping
    /// object. The value is to be converted to the native representation
    /// before returning it, if required.
    fn to_native(self) -> Raw;
}

/// ## Big-endian Encoded Values
///
/// This type represents values encoded as big-endian. It is a simple
/// wrapping-structure with the same alignment and size requirements as the
/// type it wraps.
///
/// The `FixedEndian` trait is implemented for this type if `Raw` is a
/// primitive integer. Thus, conversion from and to native endianness is
/// provided, as well as default values, ordering, and other properties
/// reliant on the native value.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct BigEndian<Raw: Copy>(Raw);

/// ## Little-endian Encoded Values
///
/// This type represents values encoded as little-endian. It is a simple
/// wrapping-structure with the same alignment and size requirements as the
/// type it wraps.
///
/// The `FixedEndian` trait is implemented for this type if `Raw` is a
/// primitive integer. Thus, conversion from and to native endianness is
/// provided, as well as default values, ordering, and other properties
/// reliant on the native value.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct LittleEndian<Raw: Copy>(Raw);

/// ## Primitive Integers
///
/// This type abstracts over primitive integers of different sizes,
/// alignments, and endianness. It is meant to be used as a replacement for
/// builtin primitive integer types like `u32` or `i64`. Unlike the builtin
/// types, this type allows working on a wide range of integers with a single
/// implementation.
///
/// Most importantly, this type allows to explicitly define its properties:
///
/// - **Size**: The size and encoding of the type matches that of `Raw`.
///
/// - **Alignment**: The alignment matches the maximum of the alignment of the
///   raw type and the alignment specified via `Alignment`.
///
/// - **Endianness*: The endianness is controlled by `Raw` and always converted
///   to native endianness when accessed via the `from/to_native()` accessors.
///
/// The non-zero property of `Raw` is propagated through this type, allowing
/// for `Option<..>` optimizations and ffi-stability.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Integer<Raw, Alignment, Native>
where
    Raw: Copy,
    Alignment: Copy,
    Native: Copy,
{
    raw: Raw,
    alignment: [Alignment; 0],
    native: core::marker::PhantomData<Native>,
}

/// ## Fixed-size non-NULL Pointers
///
/// This type is designed as alternative to `core::ptr::NonNull` but
/// provides a fixed-size address type. It allows representing 32-bit
/// pointers on 64-bit machines, and vice-versa.
///
/// Wrapping this type in an `Option<...>` is guaranteed to yield a type of
/// the same layout.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Ptr<Addr, Target>
where
    Addr: Copy,
    Target: ?Sized,
{
    addr: Addr,
    target: core::marker::PhantomData<*const Target>,
}

/// ## ABI Description
///
/// This trait defines properties of a system ABI. It provides associated types
/// for all common data-types used in a given ABI.
pub trait Abi {
    /// ZST with native alignment of the platform, used as phantom-type to
    /// raise alignment requirements of a type to the native alignment.
    type Align: Copy;
    /// ZST with alignment for 8-byte types of the platform.
    type Align8: Copy;
    /// ZST with alignment for 16-byte types of the platform.
    type Align16: Copy;
    /// ZST with alignment for 32-byte types of the platform.
    type Align32: Copy;
    /// ZST with alignment for 64-byte types of the platform.
    type Align64: Copy;
    /// ZST with alignment for 128-byte types of the platform.
    type Align128: Copy;

    /// Native address type for non-NULL values.
    type Addr: Copy;
    /// Native pointer type of the platform, pointing to a value of type
    /// `Target`.
    type Ptr<Target>: Copy;

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

    /// Big-endian signed 8-bit integer of the platform.
    type I8be;
    /// Big-endian signed 16-bit integer of the platform.
    type I16be;
    /// Big-endian signed 32-bit integer of the platform.
    type I32be;
    /// Big-endian signed 64-bit integer of the platform.
    type I64be;
    /// Big-endian signed 128-bit integer of the platform.
    type I128be;

    /// Little-endian signed 8-bit integer of the platform.
    type I8le;
    /// Little-endian signed 16-bit integer of the platform.
    type I16le;
    /// Little-endian signed 32-bit integer of the platform.
    type I32le;
    /// Little-endian signed 64-bit integer of the platform.
    type I64le;
    /// Little-endian signed 128-bit integer of the platform.
    type I128le;

    /// Native-endian signed 8-bit integer of the platform.
    type I8;
    /// Native-endian signed 16-bit integer of the platform.
    type I16;
    /// Native-endian signed 32-bit integer of the platform.
    type I32;
    /// Native-endian signed 64-bit integer of the platform.
    type I64;
    /// Native-endian signed 128-bit integer of the platform.
    type I128;

    /// Big-endian unsigned 8-bit integer of the platform.
    type U8be;
    /// Big-endian unsigned 16-bit integer of the platform.
    type U16be;
    /// Big-endian unsigned 32-bit integer of the platform.
    type U32be;
    /// Big-endian unsigned 64-bit integer of the platform.
    type U64be;
    /// Big-endian unsigned 128-bit integer of the platform.
    type U128be;

    /// Little-endian unsigned 8-bit integer of the platform.
    type U8le;
    /// Little-endian unsigned 16-bit integer of the platform.
    type U16le;
    /// Little-endian unsigned 32-bit integer of the platform.
    type U32le;
    /// Little-endian unsigned 64-bit integer of the platform.
    type U64le;
    /// Little-endian unsigned 128-bit integer of the platform.
    type U128le;

    /// Native-endian unsigned 8-bit integer of the platform.
    type U8;
    /// Native-endian unsigned 16-bit integer of the platform.
    type U16;
    /// Native-endian unsigned 32-bit integer of the platform.
    type U32;
    /// Native-endian unsigned 64-bit integer of the platform.
    type U64;
    /// Native-endian unsigned 128-bit integer of the platform.
    type U128;
}

/// ## Big-endian 32-bit ABI
pub struct Abi32be {}

/// ## Little-endian 32-bit ABI
pub struct Abi32le {}

/// ## Big-endian 64-bit ABI
pub struct Abi64be {}

/// ## Little-endian 64-bit ABI
pub struct Abi64le {}

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

// Implement `FixedEndian` on all primitive integers via identity mappings.
macro_rules! implement_endian_identity {
    ( $self:ty ) => {
        unsafe impl FixedEndian<$self> for $self {
            fn from_raw(raw: Self) -> Self { raw }
            fn to_raw(self) -> Self { self }
            fn from_native(native: Self) -> Self { native }
            fn to_native(self) -> Self { self }
        }
    }
}

implement_endian_identity!(i8);
implement_endian_identity!(i16);
implement_endian_identity!(i32);
implement_endian_identity!(i64);
implement_endian_identity!(i128);
implement_endian_identity!(isize);
implement_endian_identity!(u8);
implement_endian_identity!(u16);
implement_endian_identity!(u32);
implement_endian_identity!(u64);
implement_endian_identity!(u128);
implement_endian_identity!(usize);

impl<Addr, Target> Ptr<Addr, Target>
where
    Addr: Copy,
    Target: ?Sized,
{
    /// ## Create new instance
    ///
    /// Create a new instance of this pointer type from the provided address.
    /// The address is taken verbatim.
    #[inline]
    pub const fn new(v: Addr) -> Self {
        Self {
            addr: v,
            target: core::marker::PhantomData {},
        }
    }

    /// ## Yield underlying address
    ///
    /// Return the address underlying this pointer type.
    #[inline(always)]
    #[must_use]
    pub const fn addr(self) -> Addr {
        self.addr
    }

    /// ## Cast pointer
    ///
    /// Change the target pointer type to the specified type. This does not
    /// change the underlying address value.
    #[inline]
    pub const fn cast<OTHER>(self) -> Ptr<Addr, OTHER> {
        Ptr::<Addr, OTHER>::new(self.addr())
    }
}

// Implement clone via shallow-copy.
impl<Addr, Target> Clone for Ptr<Addr, Target>
where
    Addr: Copy,
    Target: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

// Implement copy via shallow-copy.
impl<Addr, Target> Copy for Ptr<Addr, Target>
where
    Addr: Copy,
    Target: ?Sized,
{
}

// Implement natural conversion from address to pointer.
impl<Addr, Target> From<Addr> for Ptr<Addr, Target>
where
    Addr: Copy,
    Target: ?Sized,
{
    #[inline]
    fn from(v: Addr) -> Self {
        Self::new(v)
    }
}

// Implement `Ptr` for address-types like `core::num::NonZeroU*`. This will
// provide suitable helpers to convert to and from primitive integers without
// going through the intermediate address-type.
macro_rules! implement_ptr_nonzero {
    ($addr:ty, $raw:ty) => {
        impl<Target: ?Sized> Ptr<$addr, Target> {
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
        impl<Target: ?Sized> TryFrom<$raw> for Ptr<$addr, Target> {
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
    ($addr:ty, $raw:ty) => {
        impl<Target: ?Sized> Ptr<$addr, Target> {
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

        impl<Target: ?Sized> TryFrom<usize> for Ptr<$addr, Target> {
            type Error = ();

            #[inline]
            fn try_from(v: usize) -> Result<Self, Self::Error> {
                Self::from_usize(v).ok_or(())
            }
        }

        impl<Target: Sized> Ptr<$addr, Target> {
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
                        core::mem::align_of::<Target>(),
                    )
                }
            }

            /// ## Yield address as raw pointer
            ///
            /// Return the address underlying this pointer as a raw pointer
            /// type. This pointer is guaranteed to not be NULL.
            #[inline(always)]
            #[must_use]
            pub const fn as_ptr(self) -> *const Target {
                self.as_usize() as *const Target
            }

            /// ## Yield address as raw mutable pointer
            ///
            /// Return the address underlying this pointer as a raw pointer
            /// pointer type. This pointer is guaranteed to not be NULL.
            #[inline(always)]
            #[must_use]
            pub const fn as_mut_ptr(self) -> *mut Target {
                self.as_usize() as *mut Target
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
            pub const unsafe fn as_ref<'a>(self) -> &'a Target {
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
            pub unsafe fn as_mut<'a>(self) -> &'a mut Target {
                // SAFETY: Delegated to caller.
                unsafe { &mut *(self.as_ptr() as *mut Target) }
            }
        }

        impl<Target: Sized> From<&Target> for Ptr<$addr, Target> {
            #[inline]
            fn from(v: &Target) -> Self {
                // SAFETY: References cannot be NULL.
                unsafe {
                    Self::from_raw_unchecked(
                        v as *const Target as usize as $raw,
                    )
                }
            }
        }

        impl<Target: Sized> From<&mut Target> for Ptr<$addr, Target> {
            #[inline]
            fn from(v: &mut Target) -> Self {
                // SAFETY: References cannot be NULL.
                unsafe {
                    Self::from_raw_unchecked(
                        v as *mut Target as usize as $raw,
                    )
                }
            }
        }

        impl<Target: Sized> TryFrom<*const Target> for Ptr<$addr, Target> {
            type Error = ();

            #[inline]
            fn try_from(v: *const Target) -> Result<Self, Self::Error> {
                Self::from_raw(v as usize as $raw).ok_or(())
            }
        }

        impl<Target: Sized> TryFrom<*mut Target> for Ptr<$addr, Target> {
            type Error = ();

            #[inline]
            fn try_from(v: *mut Target) -> Result<Self, Self::Error> {
                Self::from_raw(v as usize as $raw).ok_or(())
            }
        }
    }
}

// Implement `Ptr<NonZeroU*>` for common pointer sizes.
implement_ptr_nonzero!(core::num::NonZeroU32, u32);
implement_ptr_nonzero!(core::num::NonZeroU64, u64);
implement_ptr_nonzero!(core::num::NonZeroU128, u128);

// Supplement the native pointer type with converters to raw pointers et al.
#[cfg(target_pointer_width = "32")]
supplement_ptr_native!(core::num::NonZeroU32, u32);
#[cfg(target_pointer_width = "64")]
supplement_ptr_native!(core::num::NonZeroU64, u64);

impl<Raw, Alignment, Native> Integer<Raw, Alignment, Native>
where
    Raw: Copy,
    Alignment: Copy,
    Native: Copy,
{
    /// ## Create from underlying raw value
    ///
    /// Create a new integer object from its raw value. The data is taken
    /// unmodified and embedded into the new object. `to_raw()` will yield
    /// the same value again.
    ///
    /// The memory represenation of the new object is the same as of the
    /// raw value. Thus, the value can be safely transmuted instead. However,
    /// note that the alignment requirements of the data might change, so you
    /// cannot transmute pointers to the data, unless suitably aligned.
    pub fn from_raw(raw: Raw) -> Self {
        Self {
            raw: raw,
            alignment: [],
            native: core::marker::PhantomData::<Native>,
        }
    }

    /// ## Yield underlying raw value
    ///
    /// Yield the raw value that is embedded in this object. The value is
    /// returned unmodified. You can safely transmute the object to the raw
    /// type instead. See `from_raw()` for the inverse operation.
    ///
    /// Unlike `from_raw()`, you can safely transmute pointers to this object
    /// to pointers of the raw value, since the alignment requirements of `Raw`
    /// are equal to, or lower than, the alignment requirements of `Self`.
    pub fn to_raw(self) -> Raw {
        self.raw
    }

    /// ## Cast to the raw value
    ///
    /// Return a reference to the underlying raw value. It is safe to do the
    /// same via a transmute.
    pub fn as_raw(&self) -> &Raw {
        &self.raw
    }

    /// ## Cast to the mutable raw value
    ///
    /// Return a mutable reference to the underlying raw value. It is safe to
    /// do the same via a transmute.
    pub fn as_raw_mut(&mut self) -> &mut Raw {
        &mut self.raw
    }
}

// For debugging simply print the raw values.
impl<Raw, Alignment, Native> core::fmt::Debug for Integer<Raw, Alignment, Native>
where
    Raw: Copy + core::fmt::Debug,
    Alignment: Copy,
    Native: Copy,
{
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter<'_>,
    ) -> Result<(), core::fmt::Error> {
        fmt.debug_tuple("Integer")
           .field(&self.raw)
           .finish()
    }
}

impl<Raw, Alignment, Native> Integer<Raw, Alignment, Native>
where
    Raw: Copy + FixedEndian<Native>,
    Alignment: Copy,
    Native: Copy,
{
    /// ## Create from native value
    ///
    /// Create a new integer object from its native representation. This will
    /// convert the value to the representation used by the integer object. Use
    /// `to_native()` to get back the native value.
    ///
    /// If the native representation matches the raw representation, this
    /// operation is equivalent to `from_raw()`.
    pub fn from_native(v: Native) -> Self {
        Self::from_raw(Raw::from_native(v))
    }

    /// ## Convert to native value
    ///
    /// Return the native representation of the value stored in this integer.
    /// This will convert the value from the representation used by this
    /// integer object.
    ///
    /// If the native representation matches the raw representation, this
    /// operation is equivalent to `to_raw()`.
    pub fn to_native(self) -> Native {
        self.to_raw().to_native()
    }
}

// Get default from native value.
impl<Raw, Alignment, Native> Default for Integer<Raw, Alignment, Native>
where
    Raw: Copy + FixedEndian<Native>,
    Alignment: Copy,
    Native: Copy + Default,
{
    fn default() -> Self {
        Self::from_native(Default::default())
    }
}

// Convert to native for basic formatting.
impl<Raw, Alignment, Native> core::fmt::Display for Integer<Raw, Alignment, Native>
where
    Raw: Copy + FixedEndian<Native>,
    Alignment: Copy,
    Native: Copy + core::fmt::Display,
{
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter<'_>,
    ) -> Result<(), core::fmt::Error> {
        <Native as core::fmt::Display>::fmt(&self.to_native(), fmt)
    }
}

// Compare based on native value.
impl<Raw, Alignment, Native> Eq for Integer<Raw, Alignment, Native>
where
    Raw: Copy + FixedEndian<Native>,
    Alignment: Copy,
    Native: Copy + Eq,
{
}

// Import from native value.
impl<Raw, Alignment, Native> From<Native> for Integer<Raw, Alignment, Native>
where
    Raw: Copy + FixedEndian<Native>,
    Alignment: Copy,
    Native: Copy,
{
    fn from(native: Native) -> Self {
        Self::from_native(native)
    }
}

// Hash based on native value.
impl<Raw, Alignment, Native> core::hash::Hash for Integer<Raw, Alignment, Native>
where
    Raw: Copy + FixedEndian<Native>,
    Alignment: Copy,
    Native: Copy + core::hash::Hash,
{
    fn hash<Op>(&self, state: &mut Op)
    where
        Op: core::hash::Hasher,
    {
        self.to_native().hash(state)
    }
}

// Order based on native value.
impl<Raw, Alignment, Native> Ord for Integer<Raw, Alignment, Native>
where
    Raw: Copy + FixedEndian<Native>,
    Alignment: Copy,
    Native: Copy + Ord,
{
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.to_native().cmp(&other.to_native())
    }
}

// Compare based on native value.
impl<Raw, Alignment, Native> PartialEq for Integer<Raw, Alignment, Native>
where
    Raw: Copy + FixedEndian<Native>,
    Alignment: Copy,
    Native: Copy + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.to_native().eq(&other.to_native())
    }
}

// Order based on native value.
impl<Raw, Alignment, Native> PartialOrd for Integer<Raw, Alignment, Native>
where
    Raw: Copy + FixedEndian<Native>,
    Alignment: Copy,
    Native: Copy + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.to_native().partial_cmp(&other.to_native())
    }
}

// Supplement `Abi` implementations with type-aliases, which are not stable
// in trait definitions.
macro_rules! supplement_abi_aliases {
    () => {
        type Ptr<Target> = Ptr<Self::Addr, Target>;

        type Ixbe<Native: Copy, Alignment: Copy> = Integer<BigEndian<Native>, Alignment, Native>;
        type Ixle<Native: Copy, Alignment: Copy> = Integer<LittleEndian<Native>, Alignment, Native>;
        type Uxbe<Native: Copy, Alignment: Copy> = Integer<BigEndian<Native>, Alignment, Native>;
        type Uxle<Native: Copy, Alignment: Copy> = Integer<LittleEndian<Native>, Alignment, Native>;

        type I8be = Self::Ixbe<i8, Self::Align8>;
        type I16be = Self::Ixbe<i16, Self::Align16>;
        type I32be = Self::Ixbe<i32, Self::Align32>;
        type I64be = Self::Ixbe<i64, Self::Align64>;
        type I128be = Self::Ixbe<i128, Self::Align128>;

        type I8le = Self::Ixle<i8, Self::Align8>;
        type I16le = Self::Ixle<i16, Self::Align16>;
        type I32le = Self::Ixle<i32, Self::Align32>;
        type I64le = Self::Ixle<i64, Self::Align64>;
        type I128le = Self::Ixle<i128, Self::Align128>;

        type I8 = Self::Ix<i8, Self::Align8>;
        type I16 = Self::Ix<i16, Self::Align16>;
        type I32 = Self::Ix<i32, Self::Align32>;
        type I64 = Self::Ix<i64, Self::Align64>;
        type I128 = Self::Ix<i128, Self::Align128>;

        type U8be = Self::Uxbe<u8, Self::Align8>;
        type U16be = Self::Uxbe<u16, Self::Align16>;
        type U32be = Self::Uxbe<u32, Self::Align32>;
        type U64be = Self::Uxbe<u64, Self::Align64>;
        type U128be = Self::Uxbe<u128, Self::Align128>;

        type U8le = Self::Uxle<u8, Self::Align8>;
        type U16le = Self::Uxle<u16, Self::Align16>;
        type U32le = Self::Uxle<u32, Self::Align32>;
        type U64le = Self::Uxle<u64, Self::Align64>;
        type U128le = Self::Uxle<u128, Self::Align128>;

        type U8 = Self::Ux<u8, Self::Align8>;
        type U16 = Self::Ux<u16, Self::Align16>;
        type U32 = Self::Ux<u32, Self::Align32>;
        type U64 = Self::Ux<u64, Self::Align64>;
        type U128 = Self::Ux<u128, Self::Align128>;
    }
}

impl Abi for Abi32be {
    type Align = PhantomAlign32;
    type Align8 = PhantomAlign8;
    type Align16 = PhantomAlign16;
    type Align32 = Self::Align;
    type Align64 = Self::Align;
    type Align128 = Self::Align;

    type Addr = Integer<BigEndian<core::num::NonZeroU32>, Self::Align, core::num::NonZeroU32>;

    type Ix<Native: Copy, Alignment: Copy> = Self::Ixbe<Native, Alignment>;
    type Ux<Native: Copy, Alignment: Copy> = Self::Uxbe<Native, Alignment>;

    supplement_abi_aliases!();
}

impl Abi for Abi32le {
    type Align = PhantomAlign32;
    type Align8 = PhantomAlign8;
    type Align16 = PhantomAlign16;
    type Align32 = Self::Align;
    type Align64 = Self::Align;
    type Align128 = Self::Align;

    type Addr = Integer<LittleEndian<core::num::NonZeroU32>, Self::Align, core::num::NonZeroU32>;

    type Ix<Native: Copy, Alignment: Copy> = Self::Ixle<Native, Alignment>;
    type Ux<Native: Copy, Alignment: Copy> = Self::Uxle<Native, Alignment>;

    supplement_abi_aliases!();
}

impl Abi for Abi64be {
    type Align = PhantomAlign64;
    type Align8 = PhantomAlign8;
    type Align16 = PhantomAlign16;
    type Align32 = PhantomAlign32;
    type Align64 = Self::Align;
    type Align128 = Self::Align;

    type Addr = Integer<BigEndian<core::num::NonZeroU64>, Self::Align, core::num::NonZeroU64>;

    type Ix<Native: Copy, Alignment: Copy> = Self::Ixbe<Native, Alignment>;
    type Ux<Native: Copy, Alignment: Copy> = Self::Uxbe<Native, Alignment>;

    supplement_abi_aliases!();
}

impl Abi for Abi64le {
    type Align = PhantomAlign64;
    type Align8 = PhantomAlign8;
    type Align16 = PhantomAlign16;
    type Align32 = PhantomAlign32;
    type Align64 = Self::Align;
    type Align128 = Self::Align;

    type Addr = Integer<LittleEndian<core::num::NonZeroU64>, Self::Align, core::num::NonZeroU64>;

    type Ix<Native: Copy, Alignment: Copy> = Self::Ixle<Native, Alignment>;
    type Ux<Native: Copy, Alignment: Copy> = Self::Uxle<Native, Alignment>;

    supplement_abi_aliases!();
}

#[cfg(doc)]
pub type Native = Abi64le;
#[cfg(all(not(doc), target_endian = "big", target_pointer_width = "32"))]
pub type Native = Abi32be;
#[cfg(all(not(doc), target_endian = "big", target_pointer_width = "64"))]
pub type Native = Abi64be;
#[cfg(all(not(doc), target_endian = "little", target_pointer_width = "32"))]
pub type Native = Abi32le;
#[cfg(all(not(doc), target_endian = "little", target_pointer_width = "64"))]
pub type Native = Abi64le;

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

        assert_eq!(size_of::<BigEndian<i8>>(), size_of::<i8>());
        assert_eq!(align_of::<BigEndian<i8>>(), align_of::<i8>());
        assert_eq!(size_of::<BigEndian<i16>>(), size_of::<i16>());
        assert_eq!(align_of::<BigEndian<i16>>(), align_of::<i16>());
        assert_eq!(size_of::<BigEndian<i32>>(), size_of::<i32>());
        assert_eq!(align_of::<BigEndian<i32>>(), align_of::<i32>());
        assert_eq!(size_of::<BigEndian<i64>>(), size_of::<i64>());
        assert_eq!(align_of::<BigEndian<i64>>(), align_of::<i64>());
        assert_eq!(size_of::<BigEndian<i128>>(), size_of::<i128>());
        assert_eq!(align_of::<BigEndian<i128>>(), align_of::<i128>());
        assert_eq!(size_of::<LittleEndian<i8>>(), size_of::<u8>());
        assert_eq!(align_of::<LittleEndian<i8>>(), align_of::<u8>());
        assert_eq!(size_of::<LittleEndian<i16>>(), size_of::<u16>());
        assert_eq!(align_of::<LittleEndian<i16>>(), align_of::<u16>());
        assert_eq!(size_of::<LittleEndian<i32>>(), size_of::<u32>());
        assert_eq!(align_of::<LittleEndian<i32>>(), align_of::<u32>());
        assert_eq!(size_of::<LittleEndian<i64>>(), size_of::<u64>());
        assert_eq!(align_of::<LittleEndian<i64>>(), align_of::<u64>());
        assert_eq!(size_of::<LittleEndian<i128>>(), size_of::<u128>());
        assert_eq!(align_of::<LittleEndian<i128>>(), align_of::<u128>());
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

    // Verify auto traits for big/little-endian wrappers
    //
    // Verify the validity of the different auto-derived traits for the
    // `BigEndian` and `LittleEndian` type.
    #[test]
    fn endian_auto_traits() {
        // `Debug` must print the raw value.
        assert_eq!(
            std::format!("{:?}", BigEndian::<u16>(1)), "BigEndian(1)",
        );
        assert_eq!(
            std::format!("{:?}", LittleEndian::<u16>(1)), "LittleEndian(1)",
        );
    }

    // Verify `Integer` auto traits
    //
    // Verify the validity of the different auto-derived traits for the
    // `Integer` type.
    #[test]
    fn integer_auto_traits() {
        fn hash<T: core::hash::Hash>(v: T) -> u64 {
            let mut s = std::collections::hash_map::DefaultHasher::new();
            v.hash(&mut s);
            core::hash::Hasher::finish(&s)
        }

        type Test16 = Integer<u16, PhantomAlign16, u16>;

        // `Debug` must print the raw value.
        assert_eq!(std::format!("{:?}", Test16::from_raw(1)), "Integer(1)");

        // `Default` uses the native default.
        assert_eq!(<Test16 as Default>::default(), Test16::from_native(0));

        // `Display` prints the native value.
        assert_eq!(std::format!("{}", Test16::from_native(1)), "1");

        // `Eq` / `PartialEq` compare the native value.
        assert_eq!(Test16::from_native(1), Test16::from_native(1));
        assert_ne!(Test16::from_native(0), Test16::from_native(1));

        // Import from native value is supported.
        assert_eq!(Test16::from(1u16), Test16::from_native(1));

        // Hashes match the native hash.
        assert_eq!(hash(Test16::from_native(1)), hash(1u16));

        // `Ord` / `PartialOrd` compare the native value.
        assert!(Test16::from_native(0x0010) < Test16::from_native(0x0100));
    }
}
