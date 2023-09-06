//! Java Native Interface
//!
//! This Rust module exposes the JNI namespace in an architecture-independent
//! way, transposed to Rust following the rules outlined in the [`crate::ffi`]
//! module.
//!
//! Note that this module is a pure import of the definitions from the
//! specification with **no** accompanying implementation. It is suitable for
//! use in Java programs, but also in VM implementations, or other language
//! utilities.
//!
//! Extensions
//! ----------
//!
//! While this module attempts to be a direct mapping of the JNI specification,
//! slight extensions were made to account for the difference in language:
//!
//!  * `Weak` (i.e. `jweak`) is not defined in the JNI specification, yet it is
//!    included in `jni.h` of OpenJDK. It is included here as well, as it is
//!    required by several function declarations.
//!
//!  * `VmRef` and `EnvRef` are provided to wrap references to a `Vm` and `Env`
//!    type. This is so common across the entire code-base, that it makes the
//!    code a lot easier to read with the amount of `Option<NonNull<...>>`
//!    wrapping required in Rust.
//!
//! Version
//! -------
//!
//! This JNI implementation requires at least JavaSE-6 as base. Older Java
//! versions are not supported, given the lack of public information on them.
//!
//! Note that the JNI version does not necessarily equal the Java version. With
//! JavaSE-6 the JNI-1.6 was released. Hence, this is the minimum JNI version
//! supported by this module.
//!
//! For mappings from Java version to JNI version, see the JNI specification.

use crate::ffi;
use crate::ffi::util::constant;

pub trait Api {
    // Basic Types

    type Boolean: Copy;
    type Byte: Copy;
    type Char: Copy;
    type Short: Copy;
    type Int: Copy;
    type Long: Copy;
    type Float: Copy;
    type Double: Copy;

    // Objects

    type Object: Copy;
    type FieldId: Copy;
    type MethodId: Copy;

    // Aliases

    type Size: Copy;

    type Array: Copy;
    type Class: Copy;
    type String: Copy;
    type Throwable: Copy;
    type Weak: Copy;

    type BooleanArray: Copy;
    type ByteArray: Copy;
    type CharArray: Copy;
    type ShortArray: Copy;
    type IntArray: Copy;
    type LongArray: Copy;
    type FloatArray: Copy;
    type DoubleArray: Copy;
    type ObjectArray: Copy;

    // Constants

    const FALSE: Self::Boolean;
    const TRUE: Self::Boolean;

    const OK: Self::Int;
    const ERR: Self::Int;
    const EDETACHED: Self::Int;
    const EVERSION: Self::Int;
    const ENOMEM: Self::Int;
    const EEXIST: Self::Int;
    const EINVAL: Self::Int;

    const VERSION_1_1: Self::Int;
    const VERSION_1_2: Self::Int;
    const VERSION_1_4: Self::Int;
    const VERSION_1_6: Self::Int;
    const VERSION_1_8: Self::Int;
    const VERSION_9: Self::Int;
    const VERSION_10: Self::Int;
    const VERSION_19: Self::Int;
    const VERSION_20: Self::Int;
    const VERSION_21: Self::Int;

    const COMMIT: Self::Int;
    const ABORT: Self::Int;

    // Enums

    type ObjectRefType: Copy;

    const INVALID_REF_TYPE: Self::ObjectRefType;
    const LOCAL_REF_TYPE: Self::ObjectRefType;
    const GLOBAL_REF_TYPE: Self::ObjectRefType;
    const WEAK_GLOBAL_REF_TYPE: Self::ObjectRefType;

    // Structures

    type Value: Copy;
    type NativeMethod: Copy;
    type VmInitArgs: Copy;
    type VmOption: Copy;
    type VmAttachArgs: Copy;

    // Invoke Interface

    type Vm: Copy;
    type VmRef: Copy;
    type InvokeInterface: Copy;

    // Native Interface

    type Env: Copy;
    type EnvRef: Copy;
    type NativeInterface: Copy;
}

// Suppress `improper_ctypes_definitions` since we use `Option<B::Ptr<...>>`
// quite a lot, yet its zero-optimization is unstable. We verify the behavior
// in our test-suite, so as long as tests pass, this will be fine.
//
// XXX: Preferably, we would want a way to just suppress the warning for a
//      single type-alias. Unfortunately, this is not possible, so we disable
//      the lint entirely for now.
#[allow(improper_ctypes_definitions)]
pub mod api {
    use crate::{dd, ffi};
    use crate::ffi::abi::Abi;

    // Anonymous Backends

    /// Anonymous Objects
    ///
    /// This is the anonymous structure behind `jni::Object`.
    ///
    /// The JNI uses pointer types for a wide range of Java types. Yet, it
    /// avoids exposing the type of the pointee for compatibility reasons. But
    /// to guarantee type-safety for the pointers, an anonymous structure is
    /// defined for each Java type that is represented as a pointer type. This
    /// Rust implementation does the same thing, but uses explicitly named
    /// types for this, due to the lack of relevant anonymous types.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct AnonymousObject(ffi::util::Anonymous);

    /// Anonymous Field IDs
    ///
    /// The anonymous structure behind `jni::FieldID`. See `Object` for
    /// background information.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct AnonymousFieldId(ffi::util::Anonymous);

    /// Anonymous Method IDs
    ///
    /// The anonymous structure behind `jni::MethodID`. See `Object` for
    /// background information.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct AnonymousMethodId(ffi::util::Anonymous);

    // Basic Types

    pub type Boolean<B> = <B as Abi>::U8;
    pub type Byte<B> = <B as Abi>::I8;
    pub type Char<B> = <B as Abi>::U16;
    pub type Short<B> = <B as Abi>::I16;
    pub type Int<B> = <B as Abi>::I32;
    pub type Long<B> = <B as Abi>::I64;
    pub type Float<B> = <B as Abi>::F32;
    pub type Double<B> = <B as Abi>::F64;

    // Objects

    pub type Object<B> = <B as Abi>::Ptr<AnonymousObject>;
    pub type FieldId<B> = <B as Abi>::Ptr<AnonymousFieldId>;
    pub type MethodId<B> = <B as Abi>::Ptr<AnonymousMethodId>;

    // Aliases

    pub type Size<B> = Int<B>;

    pub type Array<B> = Object<B>;
    pub type Class<B> = Object<B>;
    pub type String<B> = Object<B>;
    pub type Throwable<B> = Object<B>;
    pub type Weak<B> = Object<B>;

    pub type BooleanArray<B> = Object<B>;
    pub type ByteArray<B> = Object<B>;
    pub type CharArray<B> = Object<B>;
    pub type ShortArray<B> = Object<B>;
    pub type IntArray<B> = Object<B>;
    pub type LongArray<B> = Object<B>;
    pub type FloatArray<B> = Object<B>;
    pub type DoubleArray<B> = Object<B>;
    pub type ObjectArray<B> = Object<B>;

    // Enums

    pub type ObjectRefType<B> = <B as Abi>::Enum;

    // Structures

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union Value<B: Abi> {
        pub z: Boolean<B>,
        pub b: Byte<B>,
        pub c: Char<B>,
        pub s: Short<B>,
        pub i: Int<B>,
        pub j: Long<B>,
        pub f: Float<B>,
        pub d: Double<B>,
        pub l: Object<B>,
    }

    #[repr(C)]
    #[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default, dd::Hash)]
    #[dd::derive(dd::Eq, dd::Ord, dd::PartialEq, dd::PartialOrd)]
    pub struct NativeMethod<B: Abi> {
        pub name: Option<B::Ptr<B::U8>>,
        pub signature: Option<B::Ptr<B::U8>>,
        pub fn_ptr: Option<B::Ptr<core::ffi::c_void>>,
    }

    #[repr(C)]
    #[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default, dd::Hash)]
    #[dd::derive(dd::Eq, dd::Ord, dd::PartialEq, dd::PartialOrd)]
    pub struct VmInitArgs<B: Abi> {
        pub version: Int<B>,
        pub n_options: Int<B>,
        pub options: Option<B::Ptr<VmOption<B>>>,
        pub ignore_unrecognized: Boolean<B>,
    }

    #[repr(C)]
    #[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default, dd::Hash)]
    #[dd::derive(dd::Eq, dd::Ord, dd::PartialEq, dd::PartialOrd)]
    pub struct VmOption<B: Abi> {
        pub option_string: Option<B::Ptr<B::U8>>,
        pub extra_info: Option<B::Ptr<core::ffi::c_void>>,
    }

    #[repr(C)]
    #[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default, dd::Hash)]
    #[dd::derive(dd::Eq, dd::Ord, dd::PartialEq, dd::PartialOrd)]
    pub struct VmAttachArgs<B: Abi> {
        pub version: Int<B>,
        pub name: Option<B::Ptr<B::U8>>,
        pub group: Object<B>,
    }

    // Invoke Interface

    pub type Vm<B> = Option<<B as Abi>::Ptr<InvokeInterface<B>>>;
    pub type VmRef<B> = <B as Abi>::Ptr<Vm<B>>;

    #[repr(C)]
    #[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default, dd::Hash)]
    #[dd::derive(dd::Eq, dd::Ord, dd::PartialEq, dd::PartialOrd)]
    pub struct InvokeInterface<B: Abi> {
        pub reserved0: Option<B::Ptr<core::ffi::c_void>>,
        pub reserved1: Option<B::Ptr<core::ffi::c_void>>,
        pub reserved2: Option<B::Ptr<core::ffi::c_void>>,

        pub destroy_java_vm: Option<
            unsafe extern "system" fn (
                vm: VmRef<B>,
            ) -> Int<B>,
        >,

        pub attach_current_thread: Option<
            unsafe extern "system" fn (
                vm: VmRef<B>,
                penv: B::Ptr<Option<VmRef<B>>>,
                args: Option<B::Ptr<VmAttachArgs<B>>>,
            ) -> Int<B>,
        >,

        pub detach_current_thread: Option<
            unsafe extern "system" fn (
                vm: VmRef<B>,
            ) -> Int<B>,
        >,

        pub get_env: Option<
            unsafe extern "system" fn (
                vm: VmRef<B>,
                penv: B::Ptr<Option<VmRef<B>>>,
                version: Int<B>,
            ) -> Int<B>,
        >,

        pub attach_current_thread_as_daemon: Option<
            unsafe extern "system" fn(
                vm: VmRef<B>,
                penv: B::Ptr<Option<VmRef<B>>>,
                args: Option<B::Ptr<VmAttachArgs<B>>>,
            ) -> Int<B>,
        >,
    }

    // Native Interface

    pub type Env<B> = Option<<B as Abi>::Ptr<NativeInterface<B>>>;
    pub type EnvRef<B> = <B as Abi>::Ptr<Env<B>>;

    #[repr(C)]
    #[dd::derive(dd::Clone, dd::Copy, dd::Debug, dd::Default, dd::Hash)]
    #[dd::derive(dd::Eq, dd::Ord, dd::PartialEq, dd::PartialOrd)]
    pub struct NativeInterface<B: Abi> {
        pub reserved0: Option<B::Ptr<core::ffi::c_void>>,
        pub reserved1: Option<B::Ptr<core::ffi::c_void>>,
        pub reserved2: Option<B::Ptr<core::ffi::c_void>>,
        pub reserved3: Option<B::Ptr<core::ffi::c_void>>,

        pub get_version: Option<
            unsafe extern "system" fn (
                env: EnvRef<B>,
            ) -> Int<B>,
        >,

        pub define_class: Option<B::Ptr<core::ffi::c_void>>,
        pub find_class: Option<B::Ptr<core::ffi::c_void>>,

        pub from_reflected_method: Option<B::Ptr<core::ffi::c_void>>,
        pub from_reflected_field: Option<B::Ptr<core::ffi::c_void>>,
        pub to_reflected_method: Option<B::Ptr<core::ffi::c_void>>,

        pub get_superclass: Option<B::Ptr<core::ffi::c_void>>,
        pub is_assignable_from: Option<B::Ptr<core::ffi::c_void>>,

        pub to_reflected_field: Option<B::Ptr<core::ffi::c_void>>,

        pub throw: Option<B::Ptr<core::ffi::c_void>>,
        pub throw_new: Option<B::Ptr<core::ffi::c_void>>,
        pub exception_occurred: Option<B::Ptr<core::ffi::c_void>>,
        pub exception_describe: Option<B::Ptr<core::ffi::c_void>>,
        pub exception_clear: Option<B::Ptr<core::ffi::c_void>>,
        pub fatal_error: Option<B::Ptr<core::ffi::c_void>>,

        pub push_local_frame: Option<B::Ptr<core::ffi::c_void>>,
        pub pop_local_frame: Option<B::Ptr<core::ffi::c_void>>,

        pub new_global_ref: Option<B::Ptr<core::ffi::c_void>>,
        pub delete_global_ref: Option<B::Ptr<core::ffi::c_void>>,
        pub delete_local_ref: Option<B::Ptr<core::ffi::c_void>>,
        pub is_same_object: Option<B::Ptr<core::ffi::c_void>>,
        pub new_local_ref: Option<B::Ptr<core::ffi::c_void>>,
        pub ensure_local_capacity: Option<B::Ptr<core::ffi::c_void>>,

        pub alloc_object: Option<B::Ptr<core::ffi::c_void>>,
        pub new_object: Option<B::Ptr<core::ffi::c_void>>,
        pub new_object_v: Option<B::Ptr<core::ffi::c_void>>,
        pub new_object_a: Option<B::Ptr<core::ffi::c_void>>,

        pub get_object_class: Option<B::Ptr<core::ffi::c_void>>,
        pub is_instance_of: Option<B::Ptr<core::ffi::c_void>>,

        pub get_method_id: Option<B::Ptr<core::ffi::c_void>>,

        pub missing0: [Option<B::Ptr<core::ffi::c_void>>; 30],
        pub missing1: [Option<B::Ptr<core::ffi::c_void>>; 30],

        pub get_field_id: Option<B::Ptr<core::ffi::c_void>>,

        pub missing2: [Option<B::Ptr<core::ffi::c_void>>; 18],

        pub get_static_method_id: Option<B::Ptr<core::ffi::c_void>>,

        pub missing3: [Option<B::Ptr<core::ffi::c_void>>; 18],

        pub get_static_field_id: Option<B::Ptr<core::ffi::c_void>>,

        pub missing4: [Option<B::Ptr<core::ffi::c_void>>; 9],
        pub missing5: [Option<B::Ptr<core::ffi::c_void>>; 9],

        pub new_string: Option<B::Ptr<core::ffi::c_void>>,

        pub get_string_length: Option<B::Ptr<core::ffi::c_void>>,
        pub get_string_chars: Option<B::Ptr<core::ffi::c_void>>,
        pub release_string_chars: Option<B::Ptr<core::ffi::c_void>>,

        pub new_string_utf: Option<B::Ptr<core::ffi::c_void>>,
        pub get_string_utf_length: Option<B::Ptr<core::ffi::c_void>>,
        pub get_string_utf_chars: Option<B::Ptr<core::ffi::c_void>>,
        pub release_string_utf_chars: Option<B::Ptr<core::ffi::c_void>>,

        pub get_array_length: Option<B::Ptr<core::ffi::c_void>>,

        pub new_object_array: Option<B::Ptr<core::ffi::c_void>>,
        pub get_object_array_element: Option<B::Ptr<core::ffi::c_void>>,
        pub set_object_array_element: Option<B::Ptr<core::ffi::c_void>>,

        pub missing6: [Option<B::Ptr<core::ffi::c_void>>; 8],
        pub missing7: [Option<B::Ptr<core::ffi::c_void>>; 8],
        pub missing8: [Option<B::Ptr<core::ffi::c_void>>; 8],
        pub missing9: [Option<B::Ptr<core::ffi::c_void>>; 16],

        pub register_natives: Option<B::Ptr<core::ffi::c_void>>,
        pub unregister_natives: Option<B::Ptr<core::ffi::c_void>>,

        pub monitor_enter: Option<B::Ptr<core::ffi::c_void>>,
        pub monitor_exit: Option<B::Ptr<core::ffi::c_void>>,

        pub get_java_vm: Option<B::Ptr<core::ffi::c_void>>,

        pub get_string_region: Option<B::Ptr<core::ffi::c_void>>,
        pub get_string_utf_region: Option<B::Ptr<core::ffi::c_void>>,

        pub get_primitive_array_critical: Option<B::Ptr<core::ffi::c_void>>,
        pub release_primitive_array_critical: Option<B::Ptr<core::ffi::c_void>>,

        pub get_string_critical: Option<B::Ptr<core::ffi::c_void>>,
        pub release_string_critical: Option<B::Ptr<core::ffi::c_void>>,

        pub new_weak_global_ref: Option<B::Ptr<core::ffi::c_void>>,
        pub delete_weak_global_ref: Option<B::Ptr<core::ffi::c_void>>,

        pub exception_check: Option<B::Ptr<core::ffi::c_void>>,

        pub new_direct_byte_buffer: Option<B::Ptr<core::ffi::c_void>>,
        pub get_direct_buffer_address: Option<B::Ptr<core::ffi::c_void>>,
        pub get_direct_buffer_capacity: Option<B::Ptr<core::ffi::c_void>>,

        pub get_object_ref_type: Option<B::Ptr<core::ffi::c_void>>,

        pub get_module: Option<B::Ptr<core::ffi::c_void>>,

        pub is_virtual_thread: Option<B::Ptr<core::ffi::c_void>>,
    }
}

macro_rules! implement_jni {
    ( $self:ident, $abi:ty, $cn:ident ) => {
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        struct $self {}

        impl Api for $self {
            // Basic Types

            type Boolean = api::Boolean<$abi>;
            type Byte = api::Byte<$abi>;
            type Char = api::Char<$abi>;
            type Short = api::Short<$abi>;
            type Int = api::Int<$abi>;
            type Long = api::Long<$abi>;
            type Float = api::Float<$abi>;
            type Double = api::Double<$abi>;

            // Objects

            type Object = api::Object<$abi>;
            type FieldId = api::FieldId<$abi>;
            type MethodId = api::MethodId<$abi>;

            // Aliases

            type Size = api::Size<$abi>;

            type Array = api::Array<$abi>;
            type Class = api::Class<$abi>;
            type String = api::String<$abi>;
            type Throwable = api::Throwable<$abi>;
            type Weak = api::Weak<$abi>;

            type BooleanArray = api::BooleanArray<$abi>;
            type ByteArray = api::ByteArray<$abi>;
            type CharArray = api::CharArray<$abi>;
            type DoubleArray = api::DoubleArray<$abi>;
            type FloatArray = api::FloatArray<$abi>;
            type IntArray = api::IntArray<$abi>;
            type LongArray = api::LongArray<$abi>;
            type ObjectArray = api::ObjectArray<$abi>;
            type ShortArray = api::ShortArray<$abi>;

            // Constants

            const FALSE: Self::Boolean = constant!($cn, Self::Boolean)(0);
            const TRUE: Self::Boolean = constant!($cn, Self::Boolean)(1);

            const OK: Self::Int = constant!($cn, Self::Int)(0);
            const ERR: Self::Int = constant!($cn, Self::Int)(-1);
            const EDETACHED: Self::Int = constant!($cn, Self::Int)(-2);
            const EVERSION: Self::Int = constant!($cn, Self::Int)(-3);
            const ENOMEM: Self::Int = constant!($cn, Self::Int)(-4);
            const EEXIST: Self::Int = constant!($cn, Self::Int)(-5);
            const EINVAL: Self::Int = constant!($cn, Self::Int)(-6);

            const VERSION_1_1: Self::Int = constant!($cn, Self::Int)(0x00010001);
            const VERSION_1_2: Self::Int = constant!($cn, Self::Int)(0x00010002);
            const VERSION_1_4: Self::Int = constant!($cn, Self::Int)(0x00010004);
            const VERSION_1_6: Self::Int = constant!($cn, Self::Int)(0x00010006);
            const VERSION_1_8: Self::Int = constant!($cn, Self::Int)(0x00010008);
            const VERSION_9: Self::Int = constant!($cn, Self::Int)(0x00090000);
            const VERSION_10: Self::Int = constant!($cn, Self::Int)(0x000a0000);
            const VERSION_19: Self::Int = constant!($cn, Self::Int)(0x00130000);
            const VERSION_20: Self::Int = constant!($cn, Self::Int)(0x00140000);
            const VERSION_21: Self::Int = constant!($cn, Self::Int)(0x00150000);

            const COMMIT: Self::Int = constant!($cn, Self::Int)(1);
            const ABORT: Self::Int = constant!($cn, Self::Int)(2);

            // Enums

            type ObjectRefType = api::ObjectRefType<$abi>;

            const INVALID_REF_TYPE: Self::ObjectRefType = constant!($cn, Self::ObjectRefType)(0);
            const LOCAL_REF_TYPE: Self::ObjectRefType = constant!($cn, Self::ObjectRefType)(1);
            const GLOBAL_REF_TYPE: Self::ObjectRefType = constant!($cn, Self::ObjectRefType)(2);
            const WEAK_GLOBAL_REF_TYPE: Self::ObjectRefType = constant!($cn, Self::ObjectRefType)(3);

            // Structures

            type Value = api::Value<$abi>;
            type NativeMethod = api::NativeMethod<$abi>;
            type VmInitArgs = api::VmInitArgs<$abi>;
            type VmOption = api::VmOption<$abi>;
            type VmAttachArgs = api::VmAttachArgs<$abi>;

            // Invoke Interface

            type Vm = api::Vm<$abi>;
            type VmRef = api::VmRef<$abi>;
            type InvokeInterface = api::InvokeInterface<$abi>;

            // Native Interface

            type Env = api::Env<$abi>;
            type EnvRef = api::EnvRef<$abi>;
            type NativeInterface = api::NativeInterface<$abi>;
        }
    }
}

implement_jni!(Native, ffi::abi::Native, identity);
implement_jni!(Sysv32le, ffi::abi::Sysv32le, constant);

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    // Verify ABI of types
    //
    // Check for alignments and sizes of the exposed types and verify that they
    // always match the values in the specification.
    #[test]
    fn typeinfo() {
        assert_eq!(align_of::<<Native as Api>::Boolean>(), 1);
        assert_eq!(size_of::<<Native as Api>::Boolean>(), 1);
        assert_eq!(align_of::<<Native as Api>::Byte>(), 1);
        assert_eq!(size_of::<<Native as Api>::Byte>(), 1);
        assert_eq!(align_of::<<Native as Api>::Char>(), 2);
        assert_eq!(size_of::<<Native as Api>::Char>(), 2);

        assert_eq!(align_of::<<Native as Api>::Short>(), 2);
        assert_eq!(size_of::<<Native as Api>::Short>(), 2);
        assert_eq!(align_of::<<Native as Api>::Int>(), align_of::<u32>());
        assert_eq!(size_of::<<Native as Api>::Int>(), 4);
        assert_eq!(align_of::<<Native as Api>::Long>(), align_of::<u64>());
        assert_eq!(size_of::<<Native as Api>::Long>(), 8);

        assert_eq!(align_of::<<Native as Api>::Float>(), align_of::<f32>());
        assert_eq!(size_of::<<Native as Api>::Float>(), 4);
        assert_eq!(align_of::<<Native as Api>::Double>(), align_of::<f64>());
        assert_eq!(size_of::<<Native as Api>::Double>(), 8);
    }
}
