use crate::Wizer;
use ::wasmtime::{Result, bail};

mod info;
mod instrument;
mod parse;
mod rewrite;
mod snapshot;
#[cfg(feature = "wasmtime")]
mod wasmtime;
#[cfg(feature = "wasmtime")]
pub use wasmtime::*;

const WIZER_INSTANCE: &str = "wasmtime:wizer/access";

pub use self::info::ComponentContext;

impl Wizer {
    /// Same as [`Wizer::instrument`], except for components.
    pub fn instrument_component<'a>(
        &self,
        wasm: &'a [u8],
    ) -> Result<(ComponentContext<'a>, Vec<u8>)> {
        // Make sure we're given valid Wasm from the get go.
        self.wasm_validate(&wasm)?;

        let mut cx = parse::parse(wasm)?;
        let instrumented_wasm = instrument::instrument(&mut cx)?;
        self.debug_assert_valid_wasm(&instrumented_wasm);

        Ok((cx, instrumented_wasm))
    }

    /// Same as [`Wizer::snapshot`], except for components.
    pub async fn snapshot_component(
        &self,
        mut cx: ComponentContext<'_>,
        instance: &mut impl ComponentInstanceState,
    ) -> Result<Vec<u8>> {
        if !self.func_renames.is_empty() {
            bail!("components do not support renaming functions");
        }

        let snapshot = snapshot::snapshot(&cx, instance).await;
        let rewritten_wasm = self.rewrite_component(&mut cx, &snapshot);
        self.debug_assert_valid_wasm(&rewritten_wasm);

        Ok(rewritten_wasm)
    }
}

/// Trait representing the ability to invoke functions on a component to learn
/// about its internal state.
pub trait ComponentInstanceState: Send {
    /// Looks up the exported `instance` which has `func` as an export, calls
    /// it, and returns the `list<u8>` return type.
    fn call_func_ret_list_u8(
        &mut self,
        instance: &str,
        func: &str,
        contents: impl FnOnce(&[u8]) + Send,
    ) -> impl Future<Output = ()> + Send;

    /// Same as [`Self::call_func_ret_list_u8`], but for the `s32` WIT type.
    fn call_func_ret_s32(&mut self, instance: &str, func: &str)
    -> impl Future<Output = i32> + Send;

    /// Same as [`Self::call_func_ret_list_u8`], but for the `s64` WIT type.
    fn call_func_ret_s64(&mut self, instance: &str, func: &str)
    -> impl Future<Output = i64> + Send;

    /// Same as [`Self::call_func_ret_list_u8`], but for the `f32` WIT type.
    fn call_func_ret_f32(&mut self, instance: &str, func: &str)
    -> impl Future<Output = u32> + Send;

    /// Same as [`Self::call_func_ret_list_u8`], but for the `f64` WIT type.
    fn call_func_ret_f64(&mut self, instance: &str, func: &str)
    -> impl Future<Output = u64> + Send;
}
