//! Foreign Function Interfaces
//! ===========================
//!
//! This module provides definitions from a wide range of specifications,
//! protocols, or system interfaces as native Rust data-types. The definitions
//! are transposed into Rust following strict rules and guidelines, thus
//! yielding predictable type names and definitions. No implementation of the
//! respective interfaces is provided, as this would be out of scope for this
//! module.
//!
//! Unless explicitly specified, the definitions are provided in an
//! architecture independent format. They are suitable for access of foreign
//! system architectures, as is common for introspection or debugging.
//!
//! Transpose Rules
//! ---------------
//!
//! While this module attempts to be a direct mapping to the respective
//! protocols and specifications, slight adjustments are usually necessary to
//! account for the peculiarities of Rust:
//!
//!  * All names follow the standard Rust naming scheme, using `CamelCase` for
//!    types, `UPPER_CASE` for constants, and `snake_case` for everything else.
//!
//!  * Prefixes are stripped if the Rust module or type-system provides a
//!    suitable prefix.
//!
//!  * C-enums are always provided as raw integer type, rather than Rust enum
//!    to allow arbitrary discriminants to be used. This is particularly
//!    important when the specification allows for vendor extensions, since
//!    then Rust enums would be unable to represent the vendor values.
//!
//!  * Pointers are always represented as `NonNull` or `Option<NonNull>` and
//!    thus strip any `const` annotations. This is on purpose, since the
//!    classic C-const annotations cannot be transposed to Rust in a sensible
//!    way. For architecture-independent representations, see `util::Ptr`.
//!
//! Requirements
//! ------------
//!
//! The following assumptions are made for the target platform, and verified
//! in the test-suite:
//!
//!  * The target platform uses either big-endian or little-endian encoding
//!    for multi-byte integers and addresses.
//!
//!  * The target uses either 32-bit or 64-bit wide pointers with native
//!    alignment. The `usize` type must match this size.
//!
//!  * Function pointers must be equally sized to data pointers.
//!
//!  * The c-int type is a 32-bit signed integer.
//!
//! Native Alias
//! ------------
//!
//! If suitable, a module will expose the types native to the compilation
//! target under a `native` alias (or with `*n` suffix). This allows easy
//! interaction with each module on the running system. However, it will
//! prevent any cross-architecture interaction, or interaction with non-native
//! actors.
//!
//! Addresses
//! ---------
//!
//! While the pointer-width, the size of the address-space, and offsets used
//! for pointer arithmetic do not necessarily match on every platform, the
//! interfaces implemented in this module almost exclusively encode them as
//! 32-bit and 64-bit integers, depending on the native pointer width. Hence,
//! for interfaces that deal with pointers directly, this implies that the
//! pointer-width matches the address size on the target platform. For the
//! remaining interfaces (in particular format-interfaces), this module
//! avoids using pointer types in FFI, but instead provides converters to
//! and from native pointers if they are suitably sized.
//!
//! This means, it is up to the user of these interfaces to ensure correct
//! pointer provenance, and other pointer metadata applicable to the target
//! platform.

#[cfg(not(any(
    target_endian = "big",
    target_endian = "little",
)))]
compile_error!("Target platform has an unsupported endianness.");

#[cfg(not(any(
    target_pointer_width = "32",
    target_pointer_width = "64",
)))]
compile_error!("Target platform has an unsupported pointer-width.");

pub mod abi;
pub mod util;

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    #[test]
    fn native_typeinfo() {
        assert_eq!(align_of::<*const ()>(), align_of::<usize>());
        assert_eq!(size_of::<*const ()>(), size_of::<usize>());

        assert_eq!(size_of::<*const ()>(), size_of::<*const fn()>());

        assert_eq!(align_of::<std::os::raw::c_int>(), 4);
        assert_eq!(size_of::<std::os::raw::c_int>(), 4);
    }
}
