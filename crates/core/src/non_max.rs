//! Integer newtypes that are guaranteed not to be their maximum value.

macro_rules! define_non_max {
    ( $(
        $( #[ $attr:meta ] )*
        pub struct $non_max:ident($non_zero:ty) : $prim:ty;
    )* ) => {
        $(
            /// A
            #[doc = concat!("`", stringify!($prim), "`")]
            /// value that is known not to be
            #[doc = concat!("`", stringify!($prim), "::MAX`.")]
            ///
            /// This enables some memory layout optimizations. For example,
            #[doc = concat!("`Option<", stringify!($prim), ">`")]
            /// is the same size as
            #[doc = concat!("`", stringify!($prim), "`:")]
            ///
            /// ```
            /// # use wasmtime_internal_core::non_max::NonMaxU32;
            /// assert_eq!(
            ///     core::mem::size_of::<u32>(),
            ///     core::mem::size_of::<Option<NonMaxU32>>(),
            /// );
            /// ```
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $non_max($non_zero);

            impl Default for $non_max {
                fn default() -> Self {
                    Self::new(0).unwrap()
                }
            }

            impl core::fmt::Debug for $non_max {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_tuple(stringify!($non_max))
                        .field(&self.get())
                        .finish()
                }
            }

            impl $non_max {
                /// Construct a new
                #[doc = concat!("`", stringify!($prim), "`")]
                /// value.
                ///
                /// Returns `None` when given
                #[doc = concat!("`", stringify!($prim), "::MAX`.")]
                #[inline]
                pub fn new(x: $prim) -> Option<Self> {
                    if x == <$prim>::MAX {
                        None
                    } else {
                        // Safety: `x != $prim::MAX`.
                        Some(unsafe { Self::new_unchecked(x) })
                    }
                }

                /// Unsafely construct a new
                #[doc = concat!("`", stringify!($prim), "`")]
                /// value.
                ///
                /// # Safety
                ///
                /// The given value must not be
                #[doc = concat!("`", stringify!($prim), "::MAX`.")]
                #[inline]
                pub unsafe fn new_unchecked(x: $prim) -> Self {
                    debug_assert_ne!(x, <$prim>::MAX);
                    // Safety: We know that `x+1` is non-zero because it will not
                    // overflow because `x` is not `$prim::MAX`.
                    Self(unsafe { <$non_zero>::new_unchecked(x + 1) })
                }

                /// Get the underlying
                #[doc = concat!("`", stringify!($prim), "`")]
                /// value.
                #[inline]
                pub fn get(&self) -> $prim {
                    self.0.get() - 1
                }
            }
        )*
    };
}

define_non_max! {
    pub struct NonMaxU8(core::num::NonZeroU8) : u8;
    pub struct NonMaxU16(core::num::NonZeroU16) : u16;
    pub struct NonMaxU32(core::num::NonZeroU32) : u32;
    pub struct NonMaxU64(core::num::NonZeroU64) : u64;
    pub struct NonMaxUsize(core::num::NonZeroUsize) : usize;
}
