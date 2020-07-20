use crate::wasi::wasi_ephemeral_nn::WasiEphemeralNn; // see crates/wasi/src/lib.rs
use crate::wasi::{types, Result};
use crate::WasiCtx;

impl<'a> WasiEphemeralNn for WasiCtx {
    fn load<'b>(
        &self,
        graph_buf: &wiggle::GuestPtr<'b, u8>,
        graph_buf_len: types::Size,
        encoding: types::GraphEncoding,
        target: types::ExecutionTarget,
    ) -> Result<types::Graph> {
        unimplemented!()
    }

    fn init_execution_context(&self, graph: types::Graph) -> Result<types::GraphExecutionContext> {
        unimplemented!()
    }

    fn set_input<'b>(
        &self,
        context: types::GraphExecutionContext,
        index: u32,
        tensor: &types::Tensor<'b>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn get_output<'b>(
        &self,
        context: types::GraphExecutionContext,
        index: u32,
    ) -> Result<types::Tensor<'b>> {
        unimplemented!()
    }

    fn compute(&self, context: types::GraphExecutionContext) -> Result<()> {
        unimplemented!()
    }
}
