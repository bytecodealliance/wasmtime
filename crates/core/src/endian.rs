//! Newtypes for dealing with endianness.

use core::{fmt, num::NonZero};

macro_rules! define_endian_wrapper_types {
    ( $(
        $( #[$attr:meta] )*
        pub struct $name:ident(is_little = $is_little:expr): From<$other:ident>;
    )* ) => {
        $(
            $( #[$attr] )*
            pub struct $name<T>(T);

            impl<T> From<$other<T>> for $name<T>
            where
                T: ToFromLe
            {
                #[inline]
                fn from(x: $other<T>) -> Self {
                    if Self::is_little() {
                        Self(x.get_le())
                    } else {
                        Self(x.get_ne())
                    }
                }
            }

            impl<T> Default for $name<T>
            where
                T: ToFromLe + Default
            {
                #[inline]
                fn default() -> Self {
                    Self::from_ne(T::default())
                }
            }

            impl<T> fmt::LowerHex for $name<T>
            where
                T: fmt::LowerHex + Copy + ToFromLe,
            {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::LowerHex::fmt(&self.get_ne(), f)
                }
            }

            impl<T> fmt::UpperHex for $name<T>
            where
                T: fmt::UpperHex + Copy + ToFromLe,
            {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::UpperHex::fmt(&self.get_ne(), f)
                }
            }

            impl<T> fmt::Pointer for $name<T>
            where
                T: fmt::Pointer + Copy + ToFromLe,
            {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::Pointer::fmt(&self.get_ne(), f)
                }
            }

            impl<T> $name<T> {
                #[inline]
                const fn is_little() -> bool {
                    $is_little
                }

                /// Wrap the given little-endian `T` value.
                #[inline]
                pub fn from_le(inner: T) -> Self
                where
                    T: ToFromLe,
                {
                    if Self::is_little() {
                        Self(inner)
                    } else {
                        Self(ToFromLe::from_le(inner))
                    }
                }

                /// Wrap the given native-endian `T` value.
                #[inline]
                pub fn from_ne(inner: T) -> Self
                where
                    T: ToFromLe,
                {
                    if Self::is_little() {
                        Self(ToFromLe::to_le(inner))
                    } else {
                        Self(inner)
                    }
                }

                /// Get the inner wrapped value as little-endian.
                #[inline]
                pub fn get_le(self) -> T
                where
                    T: ToFromLe,
                {
                    if Self::is_little() {
                        self.0
                    } else {
                        ToFromLe::to_le(self.0)
                    }
                }

                /// Get the inner wrapped value as native-endian.
                #[inline]
                pub fn get_ne(self) -> T
                where
                    T: ToFromLe,
                {
                    if Self::is_little() {
                        ToFromLe::from_le(self.0)
                    } else {
                        self.0
                    }
                }
            }
        )*
    };
}

define_endian_wrapper_types! {
    /// A wrapper around a native-endian `T`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[repr(transparent)]
    pub struct Ne(is_little = false): From<Le>;

    /// A wrapper around a little-endian `T`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[repr(transparent)]
    pub struct Le(is_little = true): From<Ne>;
}

/// Convert to/from little-endian.
pub trait ToFromLe {
    /// Convert from little-endian.
    fn from_le(x: Self) -> Self;
    /// Convert to little-endian.
    fn to_le(self) -> Self;
}

macro_rules! impls {
    ( $($t:ty),* $(,)? ) => {
        $(
            impl ToFromLe for $t {
                #[inline]
                fn from_le(x: Self) -> Self {
                    <$t>::from_le(x)
                }
                #[inline]
                fn to_le(self) -> Self {
                    self.to_le()
                }
            }

            impl ToFromLe for NonZero<$t> {
                #[inline]
                fn from_le(x: Self) -> Self {
                    Self::new(<$t>::from_le(x.get())).unwrap()
                }
                #[inline]
                fn to_le(self) -> Self {
                    Self::new(self.get().to_le()).unwrap()
                }
            }

            impl TryFrom<Le<$t>> for Le<NonZero<$t>> {
                type Error = <NonZero<$t> as TryFrom<$t>>::Error;

                #[inline]
                fn try_from(x: Le<$t>) -> Result<Self, Self::Error> {
                    Ok(Self::from_le(NonZero::try_from(x.get_le())?))
                }
            }

            impl TryFrom<Ne<$t>> for Ne<NonZero<$t>> {
                type Error = <NonZero<$t> as TryFrom<$t>>::Error;

                #[inline]
                fn try_from(x: Ne<$t>) -> Result<Self, Self::Error> {
                    Ok(Self::from_ne(NonZero::try_from(x.get_ne())?))
                }
            }

            impl TryFrom<Ne<$t>> for Le<NonZero<$t>> {
                type Error = <NonZero<$t> as TryFrom<$t>>::Error;

                #[inline]
                fn try_from(x: Ne<$t>) -> Result<Self, Self::Error> {
                    Ok(Self::from_le(NonZero::try_from(x.get_le())?))
                }
            }

            impl TryFrom<Le<$t>> for Ne<NonZero<$t>> {
                type Error = <NonZero<$t> as TryFrom<$t>>::Error;

                #[inline]
                fn try_from(x: Le<$t>) -> Result<Self, Self::Error> {
                    Ok(Self::from_ne(NonZero::try_from(x.get_ne())?))
                }
            }

            impl From<Le<NonZero<$t>>> for Le<$t> {
                #[inline]
                fn from(x: Le<NonZero<$t>>) -> Self {
                    Self::from_le(x.get_le().get())
                }
            }

            impl From<Ne<NonZero<$t>>> for Ne<$t> {
                #[inline]
                fn from(x: Ne<NonZero<$t>>) -> Self {
                    Self::from_ne(x.get_ne().get())
                }
            }

            impl From<Le<NonZero<$t>>> for Ne<$t> {
                #[inline]
                fn from(x: Le<NonZero<$t>>) -> Self {
                    Self::from_ne(x.get_ne().get())
                }
            }

            impl From<Ne<NonZero<$t>>> for Le<$t> {
                #[inline]
                fn from(x: Ne<NonZero<$t>>) -> Self {
                    Self::from_le(x.get_le().get())
                }
            }

            impl Le<$t> {
                /// Wrap the given little-endian bytes.
                #[inline]
                pub fn from_le_bytes(bytes: [u8; core::mem::size_of::<$t>()]) -> Self {
                    let le = <$t>::from_le_bytes(bytes);
                    Self::from_ne(le)
                }

                /// Get the wrapped value as little-endian bytes.
                #[inline]
                pub fn to_le_bytes(self) -> [u8; core::mem::size_of::<$t>()] {
                    self.get_le().to_ne_bytes()
                }

                /// Wrap the given native-endian bytes.
                #[inline]
                pub fn from_ne_bytes(bytes: [u8; core::mem::size_of::<$t>()]) -> Self {
                    let ne = <$t>::from_ne_bytes(bytes);
                    Self::from_ne(ne)
                }

                /// Get the wrapped value as native-endian bytes.
                #[inline]
                pub fn to_ne_bytes(self) -> [u8; core::mem::size_of::<$t>()] {
                    self.get_ne().to_ne_bytes()
                }
            }

            impl Ne<$t> {
                /// Wrap the given little-endian bytes.
                #[inline]
                pub fn from_le_bytes(bytes: [u8; core::mem::size_of::<$t>()]) -> Self {
                    let le = <$t>::from_le_bytes(bytes);
                    Self::from_le(le)
                }

                /// Get the wrapped value as little-endian bytes.
                #[inline]
                pub fn to_le_bytes(self) -> [u8; core::mem::size_of::<$t>()] {
                    self.get_le().to_ne_bytes()
                }

                /// Wrap the given native-endian bytes.
                #[inline]
                pub fn from_ne_bytes(bytes: [u8; core::mem::size_of::<$t>()]) -> Self {
                    let ne = <$t>::from_ne_bytes(bytes);
                    Self::from_ne(ne)
                }

                /// Get the wrapped value as native-endian bytes.
                #[inline]
                pub fn to_ne_bytes(self) -> [u8; core::mem::size_of::<$t>()] {
                    self.get_ne().to_ne_bytes()
                }
            }
        )*
    };
}

impls! {
    u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let x = Le::from_ne(0x12345678u32);
        assert_eq!(x.get_ne(), 0x12345678);

        let le_bytes = x.get_le().to_ne_bytes();
        assert_eq!(le_bytes, [0x78, 0x56, 0x34, 0x12]);

        let y = Le::from_le(x.get_le());
        assert_eq!(x, y);

        let z = Le::from_ne(x.get_ne());
        assert_eq!(x, z);
    }

    #[test]
    fn round_trip_non_zero() {
        let x = Le::from_ne(NonZero::new(0x12345678u32).unwrap());
        assert_eq!(x.get_ne().get(), 0x12345678);

        let le_bytes = x.get_le().get().to_ne_bytes();
        assert_eq!(le_bytes, [0x78, 0x56, 0x34, 0x12]);

        let y = Le::from_le(x.get_le());
        assert_eq!(x, y);

        let z = Le::from_ne(x.get_ne());
        assert_eq!(x, z);
    }
}
