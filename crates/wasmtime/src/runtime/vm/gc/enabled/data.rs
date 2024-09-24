//! Unchecked methods for working with the data inside GC objects.

use crate::V128;
use core::mem;

/// A plain-old-data type that can be stored in a `ValType` or a `StorageType`.
pub trait PodValType<const SIZE: usize>: Copy {
    /// Read an instance of `Self` from the given native-endian bytes.
    fn read_ne(ne_bytes: &[u8; SIZE]) -> Self;

    /// Write `self` into the given memory location, as native-endian bytes.
    fn write_ne(&self, into: &mut [u8; SIZE]);
}

macro_rules! impl_pod_val_type {
    ( $( $t:ty , )* ) => {
        $(
            impl PodValType<{mem::size_of::<$t>()}> for $t {
                fn read_ne(ne_bytes: &[u8; mem::size_of::<$t>()]) -> Self {
                    <$t>::from_ne_bytes(*ne_bytes)
                }
                fn write_ne(&self, into: &mut [u8; mem::size_of::<$t>()]) {
                    *into = self.to_ne_bytes();
                }
            }
        )*
    };
}

impl_pod_val_type! {
    u8,
    u16,
    u32,
    u64,
    i8,
    i16,
    i32,
    i64,
}

impl PodValType<{ mem::size_of::<V128>() }> for V128 {
    fn read_ne(ne_bytes: &[u8; mem::size_of::<V128>()]) -> Self {
        u128::from_ne_bytes(*ne_bytes).into()
    }
    fn write_ne(&self, into: &mut [u8; mem::size_of::<V128>()]) {
        *into = self.as_u128().to_ne_bytes();
    }
}

/// The backing storage for a GC-managed object.
///
/// Methods on this type do not, generally, check against things like type
/// mismatches or that the given offset to read from even falls on a field
/// boundary. Omitting these checks is memory safe, due to our untrusted,
/// indexed GC heaps. Providing incorrect offsets will result in general
/// incorrectness, such as wrong answers or even panics, however.
///
/// Finally, these methods *will* panic on out-of-bounds accesses, either out of
/// the GC heap's bounds or out of this object's bounds. The former is necessary
/// for preserving the memory safety of indexed GC heaps in the face of (for
/// example) collector bugs, but the latter is just a defensive technique to
/// catch bugs early and prevent action at a distance as much as possible.
pub struct VMGcObjectDataMut<'a> {
    data: &'a mut [u8],
}

macro_rules! impl_pod_methods {
    ( $( $t:ty, $read:ident, $write:ident; )* ) => {
        $(
            /// Read a `
            #[doc = stringify!($t)]
            /// ` field this object.
            ///
            /// Panics on out-of-bounds accesses.
            #[inline]
            pub fn $read(&self, offset: u32) -> $t {
                self.read_pod::<{ mem::size_of::<$t>() }, $t>(offset)
            }

            /// Write a `
            #[doc = stringify!($t)]
            /// ` into this object.
            ///
            /// Panics on out-of-bounds accesses.
            #[inline]
            pub fn $write(&mut self, offset: u32, val: $t) {
                self.write_pod::<{ mem::size_of::<$t>() }, $t>(offset, val);
            }
        )*
    };
}

impl<'a> VMGcObjectDataMut<'a> {
    /// Construct a `VMStructDataMut` from the given slice of bytes.
    #[inline]
    pub fn new(data: &'a mut [u8]) -> Self {
        Self { data }
    }

    /// Read a POD field out of this object.
    ///
    /// Panics on out-of-bounds accesses.
    ///
    /// Don't generally use this method, use `read_u8`, `read_i64`,
    /// etc... instead.
    #[inline]
    fn read_pod<const N: usize, T>(&self, offset: u32) -> T
    where
        T: PodValType<N>,
    {
        assert_eq!(N, mem::size_of::<T>());
        let offset = usize::try_from(offset).unwrap();
        let end = offset.checked_add(N).unwrap();
        let bytes = self.data.get(offset..end).expect("out of bounds field");
        T::read_ne(bytes.try_into().unwrap())
    }

    /// Read a POD field out of this object.
    ///
    /// Panics on out-of-bounds accesses.
    ///
    /// Don't generally use this method, use `write_u8`, `write_i64`,
    /// etc... instead.
    #[inline]
    fn write_pod<const N: usize, T>(&mut self, offset: u32, val: T)
    where
        T: PodValType<N>,
    {
        assert_eq!(N, mem::size_of::<T>());
        let offset = usize::try_from(offset).unwrap();
        let end = offset.checked_add(N).unwrap();
        let into = self.data.get_mut(offset..end).expect("out of bounds field");
        val.write_ne(into.try_into().unwrap());
    }

    impl_pod_methods! {
        u8, read_u8, write_u8;
        u16, read_u16, write_u16;
        u32, read_u32, write_u32;
        u64, read_u64, write_u64;
        i8, read_i8, write_i8;
        i16, read_i16, write_i16;
        i32, read_i32, write_i32;
        i64, read_i64, write_i64;
        V128, read_v128, write_v128;
    }
}
