//! Wasmtime compile-time intrinsic definitions.

use core::str::FromStr;

use anyhow::{Result, bail};
use serde_derive::{Deserialize, Serialize};

/// Helper macro, like `foreach_transcoder`, to iterate over builtins for
/// components unrelated to transcoding.
#[macro_export]
macro_rules! foreach_intrinsic_function {
    ($mac:ident) => {
        $mac! {
            u8_native_load(address: u64) -> u8;
            u16_native_load(address: u64) -> u16;
            u32_native_load(address: u64) -> u32;
            u64_native_load(address: u64) -> u64;

            u8_native_store(address: u64, value: u8);
            u16_native_store(address: u64, value: u16);
            u32_native_store(address: u64, value: u32);
            u64_native_store(address: u64, value: u64);

            store_data_address(vmctx: vmctx) -> u64;
        }
    };
}

// Define `struct Instrinsic`
declare_builtin_index! {
    /// An index type for Wasmtime's intrinsics available to compile-time
    /// builtins.
    #[derive(Serialize, Deserialize)]
    pub struct UnsafeIntrinsic: foreach_intrinsic_function;
}

impl FromStr for UnsafeIntrinsic {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "u8-native-load" => Self::u8_native_load(),
            "u16-native-load" => Self::u16_native_load(),
            "u32-native-load" => Self::u32_native_load(),
            "u64-native-load" => Self::u64_native_load(),

            "u8-native-store" => Self::u8_native_store(),
            "u16-native-store" => Self::u16_native_store(),
            "u32-native-store" => Self::u32_native_store(),
            "u64-native-store" => Self::u64_native_store(),

            "store-data-address" => Self::store_data_address(),

            _ => bail!("invalid Wasmtime intrinsic: {s:?}"),
        })
    }
}
