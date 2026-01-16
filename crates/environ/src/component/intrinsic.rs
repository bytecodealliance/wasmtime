//! Wasmtime compile-time intrinsic definitions.

use crate::prelude::*;
use core::str::FromStr;
use serde_derive::{Deserialize, Serialize};

/// Invoke a macro for each of our unsafe intrinsics.
#[macro_export]
macro_rules! for_each_unsafe_intrinsic {
    ($mac:ident) => {
        $mac! {
            "store-data-address" => StoreDataAddress : store_data_address() -> u64;

            "u8-native-load" => U8NativeLoad : u8_native_load(address: u64) -> u8;
            "u8-native-store" => U8NativeStore : u8_native_store(address: u64, value: u8);

            "u16-native-load" => U16NativeLoad : u16_native_load(address: u64) -> u16;
            "u16-native-store" => U16NativeStore : u16_native_store(address: u64, value: u16);

            "u32-native-load" => U32NativeLoad : u32_native_load(address: u64) -> u32;
            "u32-native-store" => U32NativeStore : u32_native_store(address: u64, value: u32);

            "u64-native-load" => U64NativeLoad : u64_native_load(address: u64) -> u64;
            "u64-native-store" => U64NativeStore : u64_native_store(address: u64, value: u64);
        }
    };
}

macro_rules! define_unsafe_intrinsics {
    (
        $(
            $symbol:expr => $variant:ident : $ctor:ident ( $( $param:ident : $param_ty:ident ),* ) $( -> $result_ty:ident )? ;
        )*
    ) => {
        /// An index type for Wasmtime's intrinsics available to compile-time
        /// builtins.
        #[repr(u32)]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub enum UnsafeIntrinsic {
            $(
                #[doc = concat!("The `", $symbol, "` intrinsic.")]
                $variant,
            )*
        }

        impl UnsafeIntrinsic {
            /// Returns the total number of unsafe intrinsics.
            pub const fn len() -> u32 {
                let mut len = 0;
                $(
                    let _ = Self::$variant;
                    len += 1;
                )*
                len
            }

            /// Construct an `UnsafeIntrinsic` from its `u32` index.
            ///
            /// Panics on invalid indices.
            pub const fn from_u32(i: u32) -> Self {
                assert!(i < Self::len());
                $(
                    if i == Self::$variant.index() {
                        return Self::$variant;
                    }
                )*
                unreachable!()
            }

            /// Get this intrinsic's index.
            pub const fn index(&self) -> u32 {
                *self as u32
            }

            /// Get this intrinsic's name.
            pub const fn name(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant => $symbol,
                    )*
                }
            }

            /// Get this intrinsic's parameters, as core Wasm value types.
            pub const fn core_params(&self) -> &'static [$crate::WasmValType] {
                match self {
                    $(
                        Self::$variant => &[ $( define_unsafe_intrinsics!(@core_type $param_ty) ),* ],
                    )*
                }
            }

            /// Get this intrinsic's results, as core Wasm value types.
            pub const fn core_results(&self) -> &'static [$crate::WasmValType] {
                match self {
                    $(
                        Self::$variant => &[ $( define_unsafe_intrinsics!(@core_type $result_ty) )? ],
                    )*
                }
            }

            /// Get this intrinsic's parameters, as component model interface types.
            pub const fn component_params(&self) -> &'static [$crate::component::InterfaceType] {
                match self {
                    $(
                        Self::$variant => &[ $( define_unsafe_intrinsics!(@component_type $param_ty) ),* ],
                    )*
                }
            }

            /// Get this intrinsic's results, as component model interface types.
            pub const fn component_results(&self) -> &'static [$crate::component::InterfaceType] {
                match self {
                    $(
                        Self::$variant => &[ $( define_unsafe_intrinsics!(@component_type $result_ty) ),* ],
                    )*
                }
            }
        }

        impl FromStr for UnsafeIntrinsic {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self> {
                match s {
                    $(
                        $symbol => Ok(Self::$variant),
                    )*
                    _ => bail!("invalid unsafe intrinsic: {s:?}"),
                }
            }
        }
    };

    (@core_type u8) => { $crate::WasmValType::I32 };
    (@core_type u16) => { $crate::WasmValType::I32 };
    (@core_type u32) => { $crate::WasmValType::I32 };
    (@core_type u64) => { $crate::WasmValType::I64 };

    (@component_type u8) => { $crate::component::InterfaceType::U8 };
    (@component_type u16) => { $crate::component::InterfaceType::U16 };
    (@component_type u32) => { $crate::component::InterfaceType::U32 };
    (@component_type u64) => { $crate::component::InterfaceType::U64 };
}

for_each_unsafe_intrinsic!(define_unsafe_intrinsics);
