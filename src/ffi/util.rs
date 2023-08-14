//! # Utility Module
//!
//! This is a utility module for the other `ffi` modules. It provides common
//! abstractions and type definitions used across many different interfaces.

use crate::mem::align;

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

/// ## Packed Data
///
/// A wrapper type that applies `repr(packed)`. This means the packed type
/// has a minimum alignment of 1, but the same size as the wrapped type.
///
/// Currently, the wrapped type must implement `Copy`, since otherwise a
/// lot of `const fn` functions cannot be used due to restrictions of the
/// Rust compiler.
#[derive(Copy, Default)]
#[repr(C, packed)]
pub struct Packed<Value: Copy>(Value);

/// ## Types as Native Addresses
///
/// This trait annotates types that effectively wrap a native memory address.
/// It allows converting the type to and from a native memory address. On top
/// of these conversions it implements a wide range of utility methods to
/// treat the type as a pointer or reference.
///
/// This type should only be implemented for types that can be represented as
/// a `usize` on the target platform. That is, this type assumes that the
/// addresses it deals with is a native memory addresses.
///
/// This is a helper trait to allow direct interaction with addresses
/// compatible with the target platform. For runtime interaction with cross
/// platform data, fallible converters must be used instead.
pub trait NativeAddress<Target>
where
    Self: Copy,
    Target: ?Sized,
{
    /// ## Create from non-zero usize
    ///
    /// Create a new instance of this type from its address given as a `usize`
    /// value. The given value must not be 0.
    ///
    /// Safety
    /// ------
    ///
    /// The caller must guarantee that the address is not zero.
    #[must_use]
    unsafe fn from_usize_unchecked(v: usize) -> Self;

    /// ## Yield address as usize
    ///
    /// Yield the address of this instance as a `usize` value. The returned
    /// address is guaranteed to be non-zero.
    #[must_use]
    fn to_usize(self) -> usize;

    /// ## Create from usize
    ///
    /// Create a new instance of this type with the address specified as a
    /// `usize` value. If the address is 0, this will yield `None`.
    #[inline]
    #[must_use]
    fn from_usize(v: usize) -> Option<Self> {
        if v == 0 {
            None
        } else {
            // SAFETY: verified to be non-zero
            unsafe { Some(Self::from_usize_unchecked(v)) }
        }
    }

    /// ## Create new dangling address
    ///
    /// Create a new instance of this type with a dangling address.
    /// This address is guaranteed to not be 0. However, the address is
    /// not necessarily unique and might match a valid address of
    /// another allocated object.
    #[inline]
    #[must_use]
    fn dangling() -> Self
    where
        Target: Sized,
    {
        // SAFETY: Alignments cannot be 0.
        unsafe {
            Self::from_usize_unchecked(
                core::mem::align_of::<Target>(),
            )
        }
    }

    /// ## Yield address as raw pointer
    ///
    /// Return the address underlying this type as a raw pointer type. This
    /// pointer is guaranteed to not be NULL.
    #[inline(always)]
    #[must_use]
    fn as_ptr(&self) -> *const Target
    where
        Target: Sized,
    {
        self.to_usize() as *const Target
    }

    /// ## Yield address as raw mutable pointer
    ///
    /// Return the address underlying this type as a raw pointer pointer type.
    /// This pointer is guaranteed to not be NULL.
    #[inline(always)]
    #[must_use]
    fn as_mut_ptr(&self) -> *mut Target
    where
        Target: Sized,
    {
        self.to_usize() as *mut Target
    }

    /// ## Yield address as reference
    ///
    /// Return the address underlying this type as a reference to the target
    /// type.
    ///
    /// Safety
    /// ------
    ///
    /// The caller must ensure that the underlying address can be safely cast
    /// into a reference, following the usual requirements of the Rust
    /// language.
    #[inline(always)]
    #[must_use]
    unsafe fn as_ref<'a>(&self) -> &'a Target
    where
        Target: Sized,
    {
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
    /// The caller must ensure that the underlying address can be safely cast
    /// into a mutable reference, following the usual requirements of the Rust
    /// language.
    #[inline(always)]
    #[must_use]
    unsafe fn as_mut<'a>(&self) -> &'a mut Target
    where
        Target: Sized,
    {
        // SAFETY: Delegated to caller.
        unsafe { &mut *self.as_mut_ptr() }
    }
}

/// ## Types with Mapping to Native Endianness
///
/// This trait allows creating instances of the implementing type from their
/// representation in native endianness, as well as converting it into native
/// endianness. If a type is already encoded in the native endianness, this
/// trait becomes an identity function for this type. For other types, it
/// converts from and to native endianness.
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
pub unsafe trait NativeEndian<Raw>
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
    #[must_use]
    fn from_raw(raw: Raw) -> Self;

    /// Return raw value
    ///
    /// Return the underlying raw, possibly foreign-ordered value behind this
    /// wrapping object. The value must be returned without any modifications.
    ///
    /// It is safe to transmute from `Self` to `Raw` instead.
    #[must_use]
    fn to_raw(self) -> Raw;

    /// Create value from native representation
    ///
    /// Create the foreign-ordered value from a native value, converting the
    /// value before retaining it, if required.
    #[must_use]
    fn from_native(native: Raw) -> Self;

    /// Return native representation
    ///
    /// Return the native representation of the value behind this wrapping
    /// object. The value is to be converted to the native representation
    /// before returning it, if required.
    #[must_use]
    fn to_native(self) -> Raw;
}

/// ## Big-endian Encoded Values
///
/// This type represents values encoded as big-endian. It is a simple
/// wrapping-structure with the same alignment and size requirements as the
/// type it wraps.
///
/// The `NativeEndian` trait is implemented for this type if `Raw` is a
/// primitive integer. Thus, conversion from and to native endianness is
/// provided, as well as default values, ordering, and other properties
/// reliant on the native value.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct BigEndian<Raw: Copy>(Raw);

/// ## Little-endian Encoded Values
///
/// This type represents values encoded as little-endian. It is a simple
/// wrapping-structure with the same alignment and size requirements as the
/// type it wraps.
///
/// The `NativeEndian` trait is implemented for this type if `Raw` is a
/// primitive integer. Thus, conversion from and to native endianness is
/// provided, as well as default values, ordering, and other properties
/// reliant on the native value.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
/// - **Alignment**: The alignment matches the maximum of the alignment of the
///   value type and the alignment specified via `Alignment`.
///
/// - **Size**: The size and encoding of the type matches that of `Value`,
///   unless the requested alignment exceeds its size. In that case, trailing
///   padding bytes are added to ensure the size is a multiple of the
///   alignment.
///
/// - **Endianness*: The endianness is controlled by `Value` and always
///   converted to native endianness when accessed via the `from/to_native()`
///   accessors.
///
/// The non-zero property of `Value` is propagated through this type, allowing
/// for `Option<..>` optimizations and ffi-stability.
#[repr(C)]
pub struct Integer<Value, Alignment, Native>
where
    Value: Copy,
    Alignment: Copy,
    Native: Copy,
{
    value: Packed<Value>,
    alignment: [Alignment; 0],
    native: core::marker::PhantomData<Native>,
}

/// ## Fixed-size Pointers
///
/// This type is designed as alternative to `core::ptr::NonNull` but
/// provides a fixed-size address type. It allows representing 32-bit
/// pointers on 64-bit machines, and vice-versa.
#[repr(transparent)]
pub struct Pointer<Address, Target>
where
    Address: Copy,
    Target: ?Sized,
{
    address: Address,
    target: core::marker::PhantomData<*const Target>,
}

/// ## Value Selector based on Address Size
///
/// Return either of the arguments, depending on the pointer-width of the
/// compilation target. For 32-bit machines `v32` is returned, for 64-bit
/// machines `v64` is returned.
#[allow(unused)]
#[must_use]
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

impl<Value> Packed<Value>
where
    Value: Copy,
{
    /// ## Create a new packed object
    ///
    /// Create a new packed object with the specified value.
    #[inline]
    #[must_use]
    pub const fn new(v: Value) -> Self {
        Self(v)
    }

    /// ## Resolve into inner value
    ///
    /// Unwrap this object and return the inner value.
    #[inline(always)]
    #[must_use]
    pub const fn into_inner(self) -> Value {
        self.0
    }

    /// ## Set to new value
    ///
    /// Change the underlying value of the wrapped type to the new value.
    /// This is equivalent to assigning a new wrapped object to this instance.
    #[inline]
    pub fn set(&mut self, v: Value) {
        self.0 = v;
    }

    /// ## Return wrapped value
    ///
    /// Return a copy of the wrapped value. The returned value will be
    /// properly aligned with all restrictions lifted.
    #[inline(always)]
    #[must_use]
    pub const fn get(&self) -> Value {
        self.0
    }

    /// ## Return pointer to unaligned value
    ///
    /// Return a pointer to the unaligned value wrapped in this
    /// packed object.
    #[inline(always)]
    #[must_use]
    pub const fn as_ptr(&self) -> *const Value {
        core::ptr::addr_of!(self.0)
    }

    /// ## Return mutable pointer to unaligned value
    ///
    /// Return a mutable pointer to the unaligned value wrapped in this
    /// packed object.
    #[inline(always)]
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut Value {
        core::ptr::addr_of_mut!(self.0)
    }
}

// Rely on `Copy` since we cannot get a reference to an unaligned value.
impl<Value> Clone for Packed<Value>
where
    Value: Copy,
{
    #[inline]
    #[must_use]
    fn clone(&self) -> Self {
        *self
    }
}

// Rely on `Copy` since we cannot get a reference to an unaligned value.
impl<Value> core::fmt::Debug for Packed<Value>
where
    Value: Copy + core::fmt::Debug,
{
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter<'_>,
    ) -> Result<(), core::fmt::Error> {
        <Value as core::fmt::Debug>::fmt(&self.get(), fmt)
    }
}

// Rely on `Copy` since we cannot get a reference to an unaligned value.
impl<Value> core::cmp::Eq for Packed<Value>
where
    Value: Copy + core::cmp::Eq,
{
}

// Rely on `Copy` since we cannot get a reference to an unaligned value.
impl<Value> core::hash::Hash for Packed<Value>
where
    Value: Copy + core::hash::Hash,
{
    fn hash<Op>(&self, state: &mut Op)
    where
        Op: core::hash::Hasher,
    {
        self.get().hash(state)
    }
}

// Rely on `Copy` since we cannot get a reference to an unaligned value.
impl<Value> core::cmp::Ord for Packed<Value>
where
    Value: Copy + core::cmp::Ord,
{
    #[must_use]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.get().cmp(&other.get())
    }
}

// Rely on `Copy` since we cannot get a reference to an unaligned value.
impl<Value> core::cmp::PartialEq for Packed<Value>
where
    Value: Copy + core::cmp::PartialEq,
{
    #[must_use]
    fn eq(&self, other: &Self) -> bool {
        self.get().eq(&other.get())
    }
}

// Rely on `Copy` since we cannot get a reference to an unaligned value.
impl<Value> core::cmp::PartialOrd for Packed<Value>
where
    Value: Copy + core::cmp::PartialOrd,
{
    #[must_use]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

// Implement `NativeAddress` on native-sized primitive integers.
macro_rules! implement_address {
    ( $self:ty ) => {
        impl<Target: ?Sized> NativeAddress<Target> for $self {
            #[inline]
            #[must_use]
            unsafe fn from_usize_unchecked(v: usize) -> Self {
                assert!(core::mem::size_of::<usize>() <= core::mem::size_of::<$self>());
                // SAFETY: as-cast never folds to 0
                v as _
            }

            #[inline(always)]
            #[must_use]
            fn to_usize(self) -> usize {
                assert!(core::mem::size_of::<$self>() <= core::mem::size_of::<usize>());
                self as _
            }
        }
    }
}

// Implement `NativeAddress` on native-sized non-zero integers.
macro_rules! implement_address_nonzero {
    ( $self:ty ) => {
        impl<Target: ?Sized> NativeAddress<Target> for $self {
            #[inline]
            #[must_use]
            unsafe fn from_usize_unchecked(v: usize) -> Self {
                assert!(core::mem::size_of::<usize>() <= core::mem::size_of::<$self>());
                // SAFETY: delegated to caller
                Self::new_unchecked(v as _)
            }

            #[inline(always)]
            #[must_use]
            fn to_usize(self) -> usize {
                assert!(core::mem::size_of::<$self>() <= core::mem::size_of::<usize>());
                self.get() as _
            }
        }
    }
}

implement_address!(usize);

#[cfg(target_pointer_width = "32")]
implement_address!(u32);
#[cfg(target_pointer_width = "64")]
implement_address!(u64);

implement_address_nonzero!(core::num::NonZeroUsize);

#[cfg(target_pointer_width = "32")]
implement_address_nonzero!(core::num::NonZeroU32);
#[cfg(target_pointer_width = "64")]
implement_address_nonzero!(core::num::NonZeroU64);

// Implement `NativeEndian` on all primitive integers via identity mappings.
macro_rules! implement_endian_identity {
    ( $self:ty ) => {
        unsafe impl NativeEndian<$self> for $self {
            #[inline]
            #[must_use]
            fn from_raw(raw: Self) -> Self { raw }

            #[inline(always)]
            #[must_use]
            fn to_raw(self) -> Self { self }

            #[inline]
            #[must_use]
            fn from_native(native: Self) -> Self { native }

            #[inline(always)]
            #[must_use]
            fn to_native(self) -> Self { self }
        }
    }
}

// Implement `NativeEndian` on big-endian integers via `from/to_be()`.
macro_rules! implement_endian_be {
    ( $self:ty, $raw:ty ) => {
        unsafe impl NativeEndian<$raw> for $self {
            #[inline]
            #[must_use]
            fn from_raw(raw: $raw) -> Self { Self(raw) }

            #[inline(always)]
            #[must_use]
            fn to_raw(self) -> $raw { self.0 }

            #[inline]
            #[must_use]
            fn from_native(native: $raw) -> Self { Self::from_raw(native.to_be()) }

            #[inline(always)]
            #[must_use]
            fn to_native(self) -> $raw { <$raw>::from_be(self.to_raw()) }
        }
    }
}

// Implement `NativeEndian` on big-endian non-zeros via `from/to_be()`.
macro_rules! implement_endian_be_nonzero {
    ( $self:ty, $raw:ty, $prim:ty ) => {
        unsafe impl NativeEndian<$raw> for $self {
            #[inline]
            #[must_use]
            fn from_raw(raw: $raw) -> Self { Self(raw) }

            #[inline(always)]
            #[must_use]
            fn to_raw(self) -> $raw { self.0 }

            #[inline]
            #[must_use]
            fn from_native(native: $raw) -> Self {
                Self::from_raw(
                    // SAFETY: endian conversion never folds to 0
                    unsafe { <$raw>::new_unchecked(native.get().to_be()) }
                )
            }

            #[inline(always)]
            #[must_use]
            fn to_native(self) -> $raw {
                // SAFETY: endian conversion never folds to 0
                unsafe { <$raw>::new_unchecked(<$prim>::from_be(self.to_raw().get())) }
            }
        }
    }
}

// Implement `NativeEndian` on little-endian integers via `from/to_le()`.
macro_rules! implement_endian_le {
    ( $self:ty, $raw:ty ) => {
        unsafe impl NativeEndian<$raw> for $self {
            #[inline]
            #[must_use]
            fn from_raw(raw: $raw) -> Self { Self(raw) }

            #[inline(always)]
            #[must_use]
            fn to_raw(self) -> $raw { self.0 }

            #[inline]
            #[must_use]
            fn from_native(native: $raw) -> Self { Self::from_raw(native.to_le()) }

            #[inline(always)]
            #[must_use]
            fn to_native(self) -> $raw { <$raw>::from_le(self.to_raw()) }
        }
    }
}

// Implement `NativeEndian` on little-endian non-zeros via `from/to_be()`.
macro_rules! implement_endian_le_nonzero {
    ( $self:ty, $raw:ty, $prim:ty ) => {
        unsafe impl NativeEndian<$raw> for $self {
            #[inline]
            #[must_use]
            fn from_raw(raw: $raw) -> Self { Self(raw) }

            #[inline(always)]
            #[must_use]
            fn to_raw(self) -> $raw { self.0 }

            #[inline]
            #[must_use]
            fn from_native(native: $raw) -> Self {
                Self::from_raw(
                    // SAFETY: endian conversion never folds to 0
                    unsafe { <$raw>::new_unchecked(native.get().to_le()) }
                )
            }

            #[inline(always)]
            #[must_use]
            fn to_native(self) -> $raw {
                // SAFETY: endian conversion never folds to 0
                unsafe { <$raw>::new_unchecked(<$prim>::from_le(self.to_raw().get())) }
            }
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
implement_endian_identity!(core::num::NonZeroI8);
implement_endian_identity!(core::num::NonZeroI16);
implement_endian_identity!(core::num::NonZeroI32);
implement_endian_identity!(core::num::NonZeroI64);
implement_endian_identity!(core::num::NonZeroI128);
implement_endian_identity!(core::num::NonZeroIsize);
implement_endian_identity!(core::num::NonZeroU8);
implement_endian_identity!(core::num::NonZeroU16);
implement_endian_identity!(core::num::NonZeroU32);
implement_endian_identity!(core::num::NonZeroU64);
implement_endian_identity!(core::num::NonZeroU128);
implement_endian_identity!(core::num::NonZeroUsize);

implement_endian_be!(BigEndian<i8>, i8);
implement_endian_be!(BigEndian<i16>, i16);
implement_endian_be!(BigEndian<i32>, i32);
implement_endian_be!(BigEndian<i64>, i64);
implement_endian_be!(BigEndian<i128>, i128);
implement_endian_be!(BigEndian<isize>, isize);
implement_endian_be!(BigEndian<u8>, u8);
implement_endian_be!(BigEndian<u16>, u16);
implement_endian_be!(BigEndian<u32>, u32);
implement_endian_be!(BigEndian<u64>, u64);
implement_endian_be!(BigEndian<u128>, u128);
implement_endian_be!(BigEndian<usize>, usize);
implement_endian_be_nonzero!(BigEndian<core::num::NonZeroI8>, core::num::NonZeroI8, i8);
implement_endian_be_nonzero!(BigEndian<core::num::NonZeroI16>, core::num::NonZeroI16, i16);
implement_endian_be_nonzero!(BigEndian<core::num::NonZeroI32>, core::num::NonZeroI32, i32);
implement_endian_be_nonzero!(BigEndian<core::num::NonZeroI64>, core::num::NonZeroI64, i64);
implement_endian_be_nonzero!(BigEndian<core::num::NonZeroI128>, core::num::NonZeroI128, i128);
implement_endian_be_nonzero!(BigEndian<core::num::NonZeroIsize>, core::num::NonZeroIsize, isize);
implement_endian_be_nonzero!(BigEndian<core::num::NonZeroU8>, core::num::NonZeroU8, u8);
implement_endian_be_nonzero!(BigEndian<core::num::NonZeroU16>, core::num::NonZeroU16, u16);
implement_endian_be_nonzero!(BigEndian<core::num::NonZeroU32>, core::num::NonZeroU32, u32);
implement_endian_be_nonzero!(BigEndian<core::num::NonZeroU64>, core::num::NonZeroU64, u64);
implement_endian_be_nonzero!(BigEndian<core::num::NonZeroU128>, core::num::NonZeroU128, u128);
implement_endian_be_nonzero!(BigEndian<core::num::NonZeroUsize>, core::num::NonZeroUsize, usize);

implement_endian_le!(LittleEndian<i8>, i8);
implement_endian_le!(LittleEndian<i16>, i16);
implement_endian_le!(LittleEndian<i32>, i32);
implement_endian_le!(LittleEndian<i64>, i64);
implement_endian_le!(LittleEndian<i128>, i128);
implement_endian_le!(LittleEndian<isize>, isize);
implement_endian_le!(LittleEndian<u8>, u8);
implement_endian_le!(LittleEndian<u16>, u16);
implement_endian_le!(LittleEndian<u32>, u32);
implement_endian_le!(LittleEndian<u64>, u64);
implement_endian_le!(LittleEndian<u128>, u128);
implement_endian_le!(LittleEndian<usize>, usize);
implement_endian_le_nonzero!(LittleEndian<core::num::NonZeroI8>, core::num::NonZeroI8, i8);
implement_endian_le_nonzero!(LittleEndian<core::num::NonZeroI16>, core::num::NonZeroI16, i16);
implement_endian_le_nonzero!(LittleEndian<core::num::NonZeroI32>, core::num::NonZeroI32, i32);
implement_endian_le_nonzero!(LittleEndian<core::num::NonZeroI64>, core::num::NonZeroI64, i64);
implement_endian_le_nonzero!(LittleEndian<core::num::NonZeroI128>, core::num::NonZeroI128, i128);
implement_endian_le_nonzero!(LittleEndian<core::num::NonZeroIsize>, core::num::NonZeroIsize, isize);
implement_endian_le_nonzero!(LittleEndian<core::num::NonZeroU8>, core::num::NonZeroU8, u8);
implement_endian_le_nonzero!(LittleEndian<core::num::NonZeroU16>, core::num::NonZeroU16, u16);
implement_endian_le_nonzero!(LittleEndian<core::num::NonZeroU32>, core::num::NonZeroU32, u32);
implement_endian_le_nonzero!(LittleEndian<core::num::NonZeroU64>, core::num::NonZeroU64, u64);
implement_endian_le_nonzero!(LittleEndian<core::num::NonZeroU128>, core::num::NonZeroU128, u128);
implement_endian_le_nonzero!(LittleEndian<core::num::NonZeroUsize>, core::num::NonZeroUsize, usize);

impl<Value, Alignment, Native> Integer<Value, Alignment, Native>
where
    Value: Copy,
    Alignment: Copy,
    Native: Copy,
{
    /// ## Create from underlying value
    ///
    /// Create a new integer object from its value. The data is taken
    /// unmodified and embedded into the new object. `value()` will yield
    /// the same value again.
    ///
    /// Note that you cannot transmute pointers to `Value` to a pointer of
    /// `Self` since `Value` might have a lower alignment than is required for
    /// `Self`.
    #[inline]
    #[must_use]
    pub const fn new(v: Value) -> Self {
        Self {
            value: Packed::new(v),
            alignment: [],
            native: core::marker::PhantomData::<Native>,
        }
    }

    /// ## Yield underlying value
    ///
    /// Yield the value that is embedded in this object. The value is
    /// returned unmodified. See `new()` for the inverse operation.
    #[inline(always)]
    #[must_use]
    pub const fn value(&self) -> Value {
        self.value.get()
    }
}

// Implement clone via propagation.
impl<Value, Alignment, Native> Clone for Integer<Value, Alignment, Native>
where
    Value: Copy,
    Alignment: Copy,
    Native: Copy,
{
    #[inline]
    #[must_use]
    fn clone(&self) -> Self {
        *self
    }
}

// Implement copy via propagation.
impl<Value, Alignment, Native> Copy for Integer<Value, Alignment, Native>
where
    Value: Copy,
    Alignment: Copy,
    Native: Copy,
{
}

// For debugging simply print the values.
impl<Value, Alignment, Native> core::fmt::Debug for Integer<Value, Alignment, Native>
where
    Value: Copy + core::fmt::Debug,
    Alignment: Copy,
    Native: Copy,
{
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter<'_>,
    ) -> Result<(), core::fmt::Error> {
        fmt.debug_tuple("Integer")
           .field(&self.value())
           .finish()
    }
}

// Propagate NativeAddress from the underlying value.
impl<Value, Alignment, Native, Target> NativeAddress<Target> for Integer<Value, Alignment, Native>
where
    Value: Copy + NativeAddress<Target>,
    Alignment: Copy,
    Native: Copy,
    Target: ?Sized,
{
    #[inline]
    #[must_use]
    unsafe fn from_usize_unchecked(v: usize) -> Self {
        Self::new(Value::from_usize_unchecked(v))
    }

    #[inline(always)]
    #[must_use]
    fn to_usize(self) -> usize {
        self.value().to_usize()
    }
}

// Propagate NativeEndian from the underlying address.
unsafe impl<Value, Alignment, Native> NativeEndian<Native> for Integer<Value, Alignment, Native>
where
    Value: Copy + NativeEndian<Native>,
    Alignment: Copy,
    Native: Copy,
{
    #[inline]
    #[must_use]
    fn from_raw(raw: Native) -> Self {
        Self::new(Value::from_raw(raw))
    }

    #[inline(always)]
    #[must_use]
    fn to_raw(self) -> Native {
        self.value().to_raw()
    }

    #[inline]
    #[must_use]
    fn from_native(native: Native) -> Self {
        Self::new(Value::from_native(native))
    }

    #[inline(always)]
    #[must_use]
    fn to_native(self) -> Native {
        self.value().to_native()
    }
}

// Get default from native value.
impl<Value, Alignment, Native> Default for Integer<Value, Alignment, Native>
where
    Value: Copy + NativeEndian<Native>,
    Alignment: Copy,
    Native: Copy + Default,
{
    fn default() -> Self {
        Self::from_native(Default::default())
    }
}

// Convert to native for basic formatting.
impl<Value, Alignment, Native> core::fmt::Display for Integer<Value, Alignment, Native>
where
    Value: Copy + NativeEndian<Native>,
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
impl<Value, Alignment, Native> Eq for Integer<Value, Alignment, Native>
where
    Value: Copy + NativeEndian<Native>,
    Alignment: Copy,
    Native: Copy + Eq,
{
}

// Import from native value.
impl<Value, Alignment, Native> From<Native> for Integer<Value, Alignment, Native>
where
    Value: Copy + NativeEndian<Native>,
    Alignment: Copy,
    Native: Copy,
{
    #[inline]
    #[must_use]
    fn from(native: Native) -> Self {
        Self::from_native(native)
    }
}

// Hash based on native value.
impl<Value, Alignment, Native> core::hash::Hash for Integer<Value, Alignment, Native>
where
    Value: Copy + NativeEndian<Native>,
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
impl<Value, Alignment, Native> Ord for Integer<Value, Alignment, Native>
where
    Value: Copy + NativeEndian<Native>,
    Alignment: Copy,
    Native: Copy + Ord,
{
    #[must_use]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.to_native().cmp(&other.to_native())
    }
}

// Compare based on native value.
impl<Value, Alignment, Native> PartialEq for Integer<Value, Alignment, Native>
where
    Value: Copy + NativeEndian<Native>,
    Alignment: Copy,
    Native: Copy + PartialEq,
{
    #[must_use]
    fn eq(&self, other: &Self) -> bool {
        self.to_native().eq(&other.to_native())
    }
}

// Order based on native value.
impl<Value, Alignment, Native> PartialOrd for Integer<Value, Alignment, Native>
where
    Value: Copy + NativeEndian<Native>,
    Alignment: Copy,
    Native: Copy + PartialOrd,
{
    #[must_use]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.to_native().partial_cmp(&other.to_native())
    }
}

impl<Address, Target> Pointer<Address, Target>
where
    Address: Copy,
    Target: ?Sized,
{
    /// ## Create new instance
    ///
    /// Create a new instance of this pointer type from the provided address.
    /// The address is taken verbatim.
    #[inline]
    #[must_use]
    pub const fn new(v: Address) -> Self {
        Self {
            address: v,
            target: core::marker::PhantomData,
        }
    }

    /// ## Yield underlying address
    ///
    /// Return the address underlying this pointer type.
    #[inline(always)]
    #[must_use]
    pub const fn address(&self) -> Address {
        self.address
    }

    /// ## Cast pointer
    ///
    /// Change the target pointer type to the specified type. This does not
    /// change the underlying address value.
    #[inline]
    #[must_use]
    pub const fn cast<Other>(&self) -> Pointer<Address, Other> {
        Pointer::<Address, Other>::new(self.address())
    }
}

// Implement clone via shallow-copy.
impl<Address, Target> Clone for Pointer<Address, Target>
where
    Address: Copy,
    Target: ?Sized,
{
    #[inline]
    #[must_use]
    fn clone(&self) -> Self {
        *self
    }
}

// Implement copy via shallow-copy.
impl<Address, Target> Copy for Pointer<Address, Target>
where
    Address: Copy,
    Target: ?Sized,
{
}

// For debugging simply print the values.
impl<Address, Target> core::fmt::Debug for Pointer<Address, Target>
where
    Address: Copy + core::fmt::Debug,
    Target: ?Sized,
{
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter<'_>,
    ) -> Result<(), core::fmt::Error> {
        fmt.debug_tuple("Pointer")
           .field(&self.address())
           .finish()
    }
}

// Ignore PhantomData for Display.
impl<Address, Target> core::fmt::Display for Pointer<Address, Target>
where
    Address: Copy + core::fmt::Display,
    Target: ?Sized,
{
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter<'_>,
    ) -> Result<(), core::fmt::Error> {
        <Address as core::fmt::Display>::fmt(&self.address(), fmt)
    }
}

// Ignore PhantomData for Eq.
impl<Address, Target> Eq for Pointer<Address, Target>
where
    Address: Copy + Eq,
    Target: ?Sized,
{
}

// Ignore PhantomData for Hash.
impl<Address, Target> core::hash::Hash for Pointer<Address, Target>
where
    Address: Copy + core::hash::Hash,
    Target: ?Sized,
{
    fn hash<Op>(&self, state: &mut Op)
    where
        Op: core::hash::Hasher,
    {
        self.address().hash(state)
    }
}

// Ignore PhantomData for Ord.
impl<Address, Target> Ord for Pointer<Address, Target>
where
    Address: Copy + Ord,
    Target: ?Sized,
{
    #[must_use]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.address().cmp(&other.address())
    }
}

// Ignore PhantomData for PartialEq.
impl<Address, Target> PartialEq for Pointer<Address, Target>
where
    Address: Copy + PartialEq,
    Target: ?Sized,
{
    #[must_use]
    fn eq(&self, other: &Self) -> bool {
        self.address().eq(&other.address())
    }
}

// Ignore PhantomData for PartialOrd.
impl<Address, Target> PartialOrd for Pointer<Address, Target>
where
    Address: Copy + PartialOrd,
    Target: ?Sized,
{
    #[must_use]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.address().partial_cmp(&other.address())
    }
}

// Propagate NativeAddress from the underlying address.
impl<Address, Target> NativeAddress<Target> for Pointer<Address, Target>
where
    Address: Copy + NativeAddress<Target>,
    Target: ?Sized,
{
    #[inline]
    #[must_use]
    unsafe fn from_usize_unchecked(v: usize) -> Self {
        Self::new(Address::from_usize_unchecked(v))
    }

    #[inline(always)]
    #[must_use]
    fn to_usize(self) -> usize {
        self.address().to_usize()
    }
}

// Implement import from usize based on NativeAddress.
impl<Address, Target> TryFrom<usize> for Pointer<Address, Target>
where
    Self: NativeAddress<Target>,
    Address: Copy,
    Target: ?Sized,
{
    type Error = ();

    fn try_from(v: usize) -> Result<Self, Self::Error> {
        Self::from_usize(v).ok_or(())
    }
}

// Implement import from reference based on NativeAddress.
impl<Address, Target> From<&Target> for Pointer<Address, Target>
where
    Self: NativeAddress<Target>,
    Address: Copy,
    Target: Sized,
{
    #[inline]
    #[must_use]
    fn from(v: &Target) -> Self {
        // SAFETY: References cannot be NULL.
        unsafe {
            Self::from_usize_unchecked(
                v as *const Target as usize,
            )
        }
    }
}

// Implement import from mutable reference based on NativeAddress.
impl<Address, Target> From<&mut Target> for Pointer<Address, Target>
where
    Self: NativeAddress<Target>,
    Address: Copy,
    Target: Sized,
{
    #[inline]
    #[must_use]
    fn from(v: &mut Target) -> Self {
        // SAFETY: References cannot be NULL.
        unsafe {
            Self::from_usize_unchecked(
                v as *mut Target as usize,
            )
        }
    }
}

// Implement import from pointer based on NativeAddress.
impl<Address, Target> TryFrom<*const Target> for Pointer<Address, Target>
where
    Self: NativeAddress<Target>,
    Address: Copy,
    Target: Sized,
{
    type Error = ();

    fn try_from(v: *const Target) -> Result<Self, Self::Error> {
        Self::from_usize(v as usize).ok_or(())
    }
}

// Implement import from pointer based on NativeAddress.
impl<Address, Target> TryFrom<*mut Target> for Pointer<Address, Target>
where
    Self: NativeAddress<Target>,
    Address: Copy,
    Target: Sized,
{
    type Error = ();

    fn try_from(v: *mut Target) -> Result<Self, Self::Error> {
        Self::from_usize(v as usize).ok_or(())
    }
}

// Propagate NativeEndian from the underlying address.
unsafe impl<Address, Target, Native> NativeEndian<Native> for Pointer<Address, Target>
where
    Address: Copy + NativeEndian<Native>,
    Target: ?Sized,
    Native: Copy,
{
    #[inline]
    #[must_use]
    fn from_raw(raw: Native) -> Self {
        Self::new(Address::from_raw(raw))
    }

    #[inline(always)]
    #[must_use]
    fn to_raw(self) -> Native {
        self.address().to_raw()
    }

    #[inline]
    #[must_use]
    fn from_native(native: Native) -> Self {
        Self::new(Address::from_native(native))
    }

    #[inline(always)]
    #[must_use]
    fn to_native(self) -> Native {
        self.address().to_native()
    }
}

// Implement `constant()` for a type
//
// Unfortunately, Rust does not allow `const fn` in traits, thus making it
// impossible to define constants of a type defined by a trait. Until Rust
// gains the required capabilities, we use a workaround: We provide the
// `constant()` function for every type used by the exported ABIs. It takes
// a suitable literal and turns it into a constant of the respective type.
//
// Note that this cannot make use of traits and trait-bounds, since those
// cannot be called from const-context. Instead, we have to channel the
// `constant()` calls upwards on each generic.
//
// Unfortunately, this requires implementing this function for every single
// combination of primitive integers with our wrapper types. This is still
// manageable, but might get unwieldy if we define more wrappers in the
// future.
macro_rules! implement_constant_as {
    ( $self:ty, $native:ty, $v:ident, $fn:expr $(,)? ) => {
        impl $self {
            /// ## Create constant instance
            ///
            /// Create a new instance suitable for constant-expressions. This
            /// is required since traits like `From` currently cannot be used
            /// in constant-expressions.
            ///
            /// This evaluates to:
            ///
            #[doc = concat!("`", stringify!($fn), "`")]
            #[doc(hidden)] // Suppress for now, to avoid many duplicates.
            pub const fn constant($v: $native) -> Self {
                $fn
            }
        }
    }
}

macro_rules! implement_constant_for_integers {
    ( $wrapper:ident, $wrapperfn:ident, $(( $native:ty, $($align:ty),+ $(,)? )),+ $(,)? ) => {
        $(
            implement_constant_as!(
                $wrapper<$native>,
                $native,
                v,
                Self(v.$wrapperfn()),
            );

            $(
                implement_constant_as!(
                    Integer<$wrapper<$native>, $align, $native>,
                    $native,
                    v,
                    Self::new($wrapper::<$native>::constant(v)),
                );
            )+
        )+
    }
}

macro_rules! implement_constant_for {
    ( $(( $native:ty, $($align:ty),+ $(,)? )),+ $(,)? ) => {
        implement_constant_for_integers!(BigEndian, to_be, $(($native, $($align),+)),+);
        implement_constant_for_integers!(LittleEndian, to_le, $(($native, $($align),+)),+);
    }
}

implement_constant_for!(
    (isize, align::AlignNative), (i8, align::Align1), (i16, align::Align2), (i32, align::Align4),
    (i64, align::Align4, align::Align8), (i128, align::Align4, align::Align8),
    (usize, align::AlignNative), (u8, align::Align1), (u16, align::Align2), (u32, align::Align4),
    (u64, align::Align4, align::Align8), (u128, align::Align4, align::Align8),
);

#[doc(hidden)]
#[macro_export]
macro_rules! ffi_util_constant {
    (identity, $type: ty) => { core::convert::identity::<$type> };
    ($fn:ident, $type: ty) => { <$type>::$fn };
}

/// ## Constant Initializer
///
/// This macro takes a selector and type as argument and evaluates to the
/// function that is used to construct such values. Possible selectors are:
///
/// - `identity`: Use `core::convert::identity::<$type>` as constructor.
/// - `$fn:ident`: Use `<$type>::$fn` as constructor.
///
/// This function does not invoke the constructor, but mere evaluates to its
/// path.
#[doc(inline)]
pub use ffi_util_constant as constant;

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
        assert_eq!(size_of::<Anonymous>(), 1);
        assert_eq!(align_of::<Anonymous>(), 1);

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

        assert_eq!(size_of::<Integer<i8, align::Align1, i8>>(), 1);
        assert_eq!(align_of::<Integer<i8, align::Align1, i8>>(), 1);
        assert_eq!(size_of::<Integer<i16, align::Align2, i16>>(), 2);
        assert_eq!(align_of::<Integer<i16, align::Align2, i16>>(), 2);
        assert_eq!(size_of::<Integer<i32, align::Align4, i32>>(), 4);
        assert_eq!(align_of::<Integer<i32, align::Align4, i32>>(), 4);
        assert_eq!(size_of::<Integer<i64, align::Align8, i64>>(), 8);
        assert_eq!(align_of::<Integer<i64, align::Align8, i64>>(), 8);
        assert_eq!(size_of::<Integer<i128, align::Align16, i128>>(), 16);
        assert_eq!(align_of::<Integer<i128, align::Align16, i128>>(), 16);
        assert_eq!(size_of::<Integer<u8, align::Align1, u8>>(), 1);
        assert_eq!(align_of::<Integer<u8, align::Align1, u8>>(), 1);
        assert_eq!(size_of::<Integer<u16, align::Align2, u16>>(), 2);
        assert_eq!(align_of::<Integer<u16, align::Align2, u16>>(), 2);
        assert_eq!(size_of::<Integer<u32, align::Align4, u32>>(), 4);
        assert_eq!(align_of::<Integer<u32, align::Align4, u32>>(), 4);
        assert_eq!(size_of::<Integer<u64, align::Align8, u64>>(), 8);
        assert_eq!(align_of::<Integer<u64, align::Align8, u64>>(), 8);
        assert_eq!(size_of::<Integer<u128, align::Align16, u128>>(), 16);
        assert_eq!(align_of::<Integer<u128, align::Align16, u128>>(), 16);

        assert_eq!(size_of::<Option<Integer<core::num::NonZeroI8, align::Align1, core::num::NonZeroI8>>>(), 1);
        assert_eq!(align_of::<Option<Integer<core::num::NonZeroI8, align::Align1, core::num::NonZeroI8>>>(), 1);
        assert_eq!(size_of::<Option<Integer<core::num::NonZeroI16, align::Align2, core::num::NonZeroI16>>>(), 2);
        assert_eq!(align_of::<Option<Integer<core::num::NonZeroI16, align::Align2, core::num::NonZeroI16>>>(), 2);
        assert_eq!(size_of::<Option<Integer<core::num::NonZeroI32, align::Align4, core::num::NonZeroI32>>>(), 4);
        assert_eq!(align_of::<Option<Integer<core::num::NonZeroI32, align::Align4, core::num::NonZeroI32>>>(), 4);
        assert_eq!(size_of::<Option<Integer<core::num::NonZeroI64, align::Align8, core::num::NonZeroI64>>>(), 8);
        assert_eq!(align_of::<Option<Integer<core::num::NonZeroI64, align::Align8, core::num::NonZeroI64>>>(), 8);
        assert_eq!(size_of::<Option<Integer<core::num::NonZeroI128, align::Align16, core::num::NonZeroI128>>>(), 16);
        assert_eq!(align_of::<Option<Integer<core::num::NonZeroI128, align::Align16, core::num::NonZeroI128>>>(), 16);
        assert_eq!(size_of::<Option<Integer<core::num::NonZeroU8, align::Align1, core::num::NonZeroU8>>>(), 1);
        assert_eq!(align_of::<Option<Integer<core::num::NonZeroU8, align::Align1, core::num::NonZeroU8>>>(), 1);
        assert_eq!(size_of::<Option<Integer<core::num::NonZeroU16, align::Align2, core::num::NonZeroU16>>>(), 2);
        assert_eq!(align_of::<Option<Integer<core::num::NonZeroU16, align::Align2, core::num::NonZeroU16>>>(), 2);
        assert_eq!(size_of::<Option<Integer<core::num::NonZeroU32, align::Align4, core::num::NonZeroU32>>>(), 4);
        assert_eq!(align_of::<Option<Integer<core::num::NonZeroU32, align::Align4, core::num::NonZeroU32>>>(), 4);
        assert_eq!(size_of::<Option<Integer<core::num::NonZeroU64, align::Align8, core::num::NonZeroU64>>>(), 8);
        assert_eq!(align_of::<Option<Integer<core::num::NonZeroU64, align::Align8, core::num::NonZeroU64>>>(), 8);
        assert_eq!(size_of::<Option<Integer<core::num::NonZeroU128, align::Align16, core::num::NonZeroU128>>>(), 16);
        assert_eq!(align_of::<Option<Integer<core::num::NonZeroU128, align::Align16, core::num::NonZeroU128>>>(), 16);
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

        type Test16 = Integer<u16, align::Align2, u16>;

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

    // Verify `Integer` advanced type layout
    //
    // Check some non-standard type-parameters for `Integer` and verify the
    // memory layout.
    #[test]
    fn integer_typeinfo() {
        // verify that the alignment honors the request
        assert_eq!(align_of::<Integer<u8, align::Align16, u8>>(), 16);
        assert_eq!(align_of::<Integer<u128, align::Align1, u128>>(), 1);

        // verify that high alignments cause padding
        assert_eq!(size_of::<Integer<u8, align::Align16, u8>>(), 16);

        // zero-optimization must propagate through `Integer<BigEndian<...>>`
        assert_eq!(
            size_of::<Option<Integer<BigEndian<core::num::NonZeroI64>, align::Align8, core::num::NonZeroI64>>>(),
            8,
        );
        assert_eq!(
            align_of::<Option<Integer<BigEndian<core::num::NonZeroI64>, align::Align8, core::num::NonZeroI64>>>(),
            8,
        );
    }

    // Verify `Pointer` advanced type layout
    //
    // Check some non-standard combinations for `Pointer` and verify that the
    // layout is properly defined.
    #[test]
    fn pointer_typeinfo() {
        // zero-optimization must propagate through `Pointer<Integer<BigEndian<...>>>`
        assert_eq!(
            size_of::<Option<Pointer<
                Integer<BigEndian<core::num::NonZeroI64>, align::Align8, core::num::NonZeroI64>,
                u8,
            >>>(),
            8,
        );
        assert_eq!(
            align_of::<Option<Pointer<
                Integer<BigEndian<core::num::NonZeroI64>, align::Align8, core::num::NonZeroI64>,
                u8,
            >>>(),
            8,
        );
    }
}
