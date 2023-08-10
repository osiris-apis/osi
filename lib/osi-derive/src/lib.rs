//! # Direct Derive
//!
//! This proc-macro crate provides an alternative
//! [`crate::derive`](macro@crate::derive) attribute, which generates trait
//! implementations for a type based solely on its structural layout. It works
//! very similar to the [`core::derive`](core::prelude::v1::derive) attribute
//! of the Rust standard library, but uses direct trait-bounds on each field
//! rather than putting a trait-bound on each generic argument (known as
//! _perfect derive_).
//!
//! The attribute has builtin support to derive many of the core Rust traits,
//! but allows for external code to derive any custom trait via simple Rust
//! macro definitions.
//!
//! ## Example
//!
//! The following example shows how to derive `PartialEq` for a type that uses
//! associated trait types. The standard Rust `derive` attribute would place a
//! trait bound `F: Debug`, while this macro only places the bound
//! `F::Output: Debug`, since this is the actual bound required.
//!
//! ```rust
//! use osi_derive as dd;
//!
//! #[dd::derive(dd::Debug)]
//! struct Output<F: std::future::Future> {
//!     output: Option<F::Output>,
//! }
//! ```

// XXX:
// - Move each derive-operation into a separate module so we can add proper
//   tests. These should go beyond syntactical tests, and rather verify the
//   behavior of the structural derive.

#[doc(hidden)]
#[macro_export]
macro_rules! derive_clone_inner {
    (
        $ident: ident,
        $type: ty,
        ($($generics: tt)*),
        ($( ($($where: tt)*) ),* $(,)?),
        ($($field_ident: tt),* $(,)?),
        ($($field_type: ty),* $(,)?)
        $(,)?
    ) => {
        impl $($generics)* ::core::clone::Clone for $type
        where
            $($($where)*,)*
            $($field_type: ::core::clone::Clone,)*
        {
            fn clone(&self) -> Self {
                Self {
                    $(
                        $field_ident: self.$field_ident.clone(),
                    )*
                }
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_clone {
    (derive_struct, $($tt: tt)*) => {
        $crate::derive_clone_inner!{$($tt)*}
    };
    (derive_tuple, $($tt: tt)*) => {
        $crate::derive_clone_inner!{$($tt)*}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_copy_inner {
    (
        $ident: ident,
        $type: ty,
        ($($generics: tt)*),
        ($( ($($where: tt)*) ),* $(,)?),
        ($($field_ident: tt),* $(,)?),
        ($($field_type: ty),* $(,)?)
        $(,)?
    ) => {
        impl $($generics)* ::core::marker::Copy for $type
        where
            $($($where)*,)*
            $($field_type: ::core::marker::Copy,)*
        {
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_copy {
    (derive_struct, $($tt: tt)*) => {
        $crate::derive_copy_inner!{$($tt)*}
    };
    (derive_tuple, $($tt: tt)*) => {
        $crate::derive_copy_inner!{$($tt)*}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_debug_inner {
    (
        derive_struct,
        $ident: ident,
        $type: ty,
        ($($generics: tt)*),
        ($( ($($where: tt)*) ),* $(,)?),
        ($($field_ident: tt),* $(,)?),
        ($($field_type: ty),* $(,)?)
        $(,)?
    ) => {
        impl $($generics)* ::core::fmt::Debug for $type
        where
            $($($where)*,)*
            $($field_type: ::core::fmt::Debug,)*
        {
            fn fmt(
                &self,
                fmt: &mut ::core::fmt::Formatter<'_>,
            ) -> ::core::fmt::Result {
                fmt
                    .debug_struct(::core::stringify!($ident))
                    $(
                        .field(
                            ::core::stringify!($field_ident),
                            &self.$field_ident,
                        )
                    )*
                    .finish()
            }
        }
    };
    (
        derive_tuple,
        $ident: ident,
        $type: ty,
        ($($generics: tt)*),
        ($( ($($where: tt)*) ),* $(,)?),
        ($($field_ident: tt),* $(,)?),
        ($($field_type: ty),* $(,)?)
        $(,)?
    ) => {
        impl $($generics)* ::core::fmt::Debug for $type
        where
            $($($where)*,)*
            $($field_type: ::core::fmt::Debug,)*
        {
            fn fmt(
                &self,
                fmt: &mut ::core::fmt::Formatter<'_>,
            ) -> ::core::fmt::Result {
                fmt
                    .debug_tuple(::core::stringify!($ident))
                    $(.field(&self.$field_ident))*
                    .finish()
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_debug {
    (derive_struct, $($tt: tt)*) => {
        $crate::derive_debug_inner!{derive_struct, $($tt)*}
    };
    (derive_tuple, $($tt: tt)*) => {
        $crate::derive_debug_inner!{derive_tuple, $($tt)*}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_default_inner {
    (
        $ident: ident,
        $type: ty,
        ($($generics: tt)*),
        ($( ($($where: tt)*) ),* $(,)?),
        ($($field_ident: tt),* $(,)?),
        ($($field_type: ty),* $(,)?)
        $(,)?
    ) => {
        impl $($generics)* ::core::default::Default for $type
        where
            $($($where)*,)*
            $($field_type: ::core::default::Default,)*
        {
            fn default() -> Self {
                Self {
                    $(
                        $field_ident: <_ as ::core::default::Default>::default(),
                    )*
                }
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_default {
    (derive_struct, $($tt: tt)*) => {
        $crate::derive_default_inner!{$($tt)*}
    };
    (derive_tuple, $($tt: tt)*) => {
        $crate::derive_default_inner!{$($tt)*}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_eq_inner {
    (
        $ident: ident,
        $type: ty,
        ($($generics: tt)*),
        ($( ($($where: tt)*) ),* $(,)?),
        ($($field_ident: tt),* $(,)?),
        ($($field_type: ty),* $(,)?)
        $(,)?
    ) => {
        impl $($generics)* ::core::cmp::Eq for $type
        where
            $($($where)*,)*
            $($field_type: ::core::cmp::Eq,)*
        {
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_eq {
    (derive_struct, $($tt: tt)*) => {
        $crate::derive_eq_inner!{$($tt)*}
    };
    (derive_tuple, $($tt: tt)*) => {
        $crate::derive_eq_inner!{$($tt)*}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_hash_inner {
    (
        $ident: ident,
        $type: ty,
        ($($generics: tt)*),
        ($( ($($where: tt)*) ),* $(,)?),
        ($($field_ident: tt),* $(,)?),
        ($($field_type: ty),* $(,)?)
        $(,)?
    ) => {
        impl $($generics)* ::core::hash::Hash for $type
        where
            $($($where)*,)*
            $($field_type: ::core::hash::Hash,)*
        {
            #[allow(unused_variables)]
            fn hash<Op>(&self, state: &mut Op)
            where
                Op: ::core::hash::Hasher,
            {
                $(
                    self.$field_ident.hash(state);
                )*
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_hash {
    (derive_struct, $($tt: tt)*) => {
        $crate::derive_hash_inner!{$($tt)*}
    };
    (derive_tuple, $($tt: tt)*) => {
        $crate::derive_hash_inner!{$($tt)*}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_ord_inner {
    (
        $ident: ident,
        $type: ty,
        ($($generics: tt)*),
        ($( ($($where: tt)*) ),* $(,)?),
        ($($field_ident: tt),* $(,)?),
        ($($field_type: ty),* $(,)?)
        $(,)?
    ) => {
        impl $($generics)* ::core::cmp::Ord for $type
        where
            $($($where)*,)*
            $($field_type: ::core::cmp::Ord,)*
        {
            #[allow(unused_variables)]
            fn cmp(
                &self,
                other: &Self,
            ) -> ::core::cmp::Ordering {
                ::core::cmp::Ordering::Equal
                    $(
                        .then(
                            <_ as ::core::cmp::Ord>::cmp(
                                &self.$field_ident,
                                &other.$field_ident,
                            )
                        )
                    )*
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_ord {
    (derive_struct, $($tt: tt)*) => {
        $crate::derive_ord_inner!{$($tt)*}
    };
    (derive_tuple, $($tt: tt)*) => {
        $crate::derive_ord_inner!{$($tt)*}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_partialeq_inner {
    (
        $ident: ident,
        $type: ty,
        ($($generics: tt)*),
        ($( ($($where: tt)*) ),* $(,)?),
        ($($field_ident: tt),* $(,)?),
        ($($field_type: ty),* $(,)?)
        $(,)?
    ) => {
        impl $($generics)* ::core::cmp::PartialEq for $type
        where
            $($($where)*,)*
            $($field_type: ::core::cmp::PartialEq,)*
        {
            #[allow(unused_variables)]
            fn eq(&self, other: &Self) -> bool {
                true
                $(
                    && <_ as ::core::cmp::PartialEq>::eq(
                        &self.$field_ident,
                        &other.$field_ident,
                    )
                )*
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_partialeq {
    (derive_struct, $($tt: tt)*) => {
        $crate::derive_partialeq_inner!{$($tt)*}
    };
    (derive_tuple, $($tt: tt)*) => {
        $crate::derive_partialeq_inner!{$($tt)*}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_partialord_inner {
    (
        $ident: ident,
        $type: ty,
        ($($generics: tt)*),
        ($( ($($where: tt)*) ),* $(,)?),
        ($($field_ident: tt),* $(,)?),
        ($($field_type: ty),* $(,)?)
        $(,)?
    ) => {
        impl $($generics)* ::core::cmp::PartialOrd for $type
        where
            $($($where)*,)*
            $($field_type: ::core::cmp::PartialOrd,)*
        {
            #[allow(unused_variables)]
            fn partial_cmp(
                &self,
                other: &Self,
            ) -> Option<::core::cmp::Ordering> {
                Some(::core::cmp::Ordering::Equal)
                    $(
                        .and_then(
                            |v| match v {
                                ::core::cmp::Ordering::Equal => {
                                    <_ as ::core::cmp::PartialOrd>::partial_cmp(
                                        &self.$field_ident,
                                        &other.$field_ident,
                                    )
                                },
                                _ => Some(v),
                            },
                        )
                    )*
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! derive_partialord {
    (derive_struct, $($tt: tt)*) => {
        $crate::derive_partialord_inner!{$($tt)*}
    };
    (derive_tuple, $($tt: tt)*) => {
        $crate::derive_partialord_inner!{$($tt)*}
    };
}

// Expose the attribute from the underlying proc-crate.
#[doc(inline)]
pub use osi_derive_proc::derive as derive;

/// ## Direct Derive of [`core::clone::Clone`]
///
/// This derives [`core::clone::Clone`] for the target type,
/// if used via [`derive`].
pub use derive_clone as Clone;

/// ## Direct Derive of [`core::marker::Copy`]
///
/// This derives [`core::marker::Copy`] for the target type,
/// if used via [`derive`].
pub use derive_copy as Copy;

/// ## Direct Derive of [`core::default::Default`]
///
/// This derives [`core::default::Default`] for the target type,
/// if used via [`derive`].
pub use derive_default as Default;

/// ## Direct Derive of [`core::cmp::Eq`]
///
/// This derives [`core::cmp::Eq`] for the target type,
/// if used via [`derive`].
pub use derive_eq as Eq;

/// ## Direct Derive of [`core::hash::Hash`]
///
/// This derives [`core::hash::Hash`] for the target type,
/// if used via [`derive`].
pub use derive_hash as Hash;

/// ## Direct Derive of [`core::cmp::Ord`]
///
/// This derives [`core::cmp::Ord`] for the target type,
/// if used via [`derive`].
pub use derive_ord as Ord;

/// ## Direct Derive of [`core::fmt::Debug`]
///
/// This derives [`core::fmt::Debug`] for the target type,
/// if used via [`derive`].
///
/// The implementation uses the `debug_*` functions of
/// [`core::fmt::Formatter`] to visualize the target type.
#[doc(inline)]
pub use derive_debug as Debug;

/// ## Direct Derive of [`core::cmp::PartialEq`]
///
/// This derives [`core::cmp::PartialEq`] for the target type,
/// if used via [`derive`].
#[doc(inline)]
pub use derive_partialeq as PartialEq;

/// ## Direct Derive of [`core::cmp::PartialOrd`]
///
/// This derives [`core::cmp::PartialOrd`] for the target type,
/// if used via [`derive`].
#[doc(inline)]
pub use derive_partialord as PartialOrd;

#[cfg(test)]
mod tests {
    use crate as dd;

    #[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default)]
    #[dd::derive(dd::Eq, dd::Hash, dd::Ord)]
    #[dd::derive(dd::PartialEq, dd::PartialOrd)]
    struct TestStruct0 {
    }

    #[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default)]
    #[dd::derive(dd::Eq, dd::Hash, dd::Ord)]
    #[dd::derive(dd::PartialEq, dd::PartialOrd)]
    struct TestStruct1 {
        a: u8,
    }

    #[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default)]
    #[dd::derive(dd::Eq, dd::Hash, dd::Ord)]
    #[dd::derive(dd::PartialEq, dd::PartialOrd)]
    struct TestStruct2 {
        a: u8,
        b: u16,
    }

    #[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default)]
    #[dd::derive(dd::Eq, dd::Hash, dd::Ord)]
    #[dd::derive(dd::PartialEq, dd::PartialOrd)]
    struct TestTuple0();

    #[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default)]
    #[dd::derive(dd::Eq, dd::Hash, dd::Ord)]
    #[dd::derive(dd::PartialEq, dd::PartialOrd)]
    struct TestTuple1(u8);

    #[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default)]
    #[dd::derive(dd::Eq, dd::Hash, dd::Ord)]
    #[dd::derive(dd::PartialEq, dd::PartialOrd)]
    struct TestTuple2(u8, u16);

    #[test]
    fn instantiation() {
        let _: TestStruct0 = Default::default();
        let _: TestStruct1 = Default::default();
        let _: TestStruct2 = Default::default();
        let _: TestTuple0 = Default::default();
        let _: TestTuple1 = Default::default();
        let _: TestTuple2 = Default::default();
    }
}
