//! Implements the wasi-nn API.
use crate::ctx::WasiNnResult as Result;
use crate::witx::types::{
    ExecutionTarget, Graph, GraphBuilderArray, GraphEncoding, GraphExecutionContext, Tensor,
};
use crate::witx::wasi_ephemeral_nn::WasiEphemeralNn;
use crate::WasiNnCtx;
use thiserror::Error;
use wiggle::GuestPtr;

#[derive(Debug, Error)]
pub enum UsageError {
    #[error("Invalid context; has the load function been called?")]
    InvalidContext,
    #[error("Only OpenVINO's IR is currently supported, passed encoding: {0:?}")]
    InvalidEncoding(GraphEncoding),
    #[error("OpenVINO expects only two buffers (i.e. [ir, weights]), passed: {0}")]
    InvalidNumberOfBuilders(u32),
    #[error("Invalid graph handle; has it been loaded?")]
    InvalidGraphHandle,
    #[error("Invalid execution context handle; has it been initialized?")]
    InvalidExecutionContextHandle,
    #[error("Not enough memory to copy tensor data of size: {0}")]
    NotEnoughMemory(u32),
}

impl<'a> WasiEphemeralNn for WasiNnCtx {
    fn load<'b>(
        &mut self,
        builders: &GraphBuilderArray<'_>,
        encoding: GraphEncoding,
        target: ExecutionTarget,
    ) -> Result<Graph> {
        let encoding_id: u8 = encoding.into();
        let graph = if let Some(backend) = self.ctx.borrow_mut().backends.get_mut(&encoding_id) {
            backend.load(builders, target)?
        } else {
            return Err(UsageError::InvalidEncoding(encoding).into());
        };
        let graph_id = self.ctx.borrow_mut().graphs.insert(graph);
        Ok(graph_id)
    }

    fn init_execution_context(&mut self, graph_id: Graph) -> Result<GraphExecutionContext> {
        let exec_context = if let Some(graph) = self.ctx.borrow_mut().graphs.get_mut(graph_id) {
            graph.init_execution_context()?
        } else {
            return Err(UsageError::InvalidGraphHandle.into());
        };

        let exec_context_id = self.ctx.borrow_mut().executions.insert(exec_context);
        Ok(exec_context_id)
    }

    fn set_input<'b>(
        &mut self,
        exec_context_id: GraphExecutionContext,
        index: u32,
        tensor: &Tensor<'b>,
    ) -> Result<()> {
        if let Some(exec_context) = self.ctx.borrow_mut().executions.get_mut(exec_context_id) {
            Ok(exec_context.set_input(index, tensor)?)
        } else {
            Err(UsageError::InvalidGraphHandle.into())
        }
    }

    fn compute(&mut self, exec_context_id: GraphExecutionContext) -> Result<()> {
        if let Some(exec_context) = self.ctx.borrow_mut().executions.get_mut(exec_context_id) {
            Ok(exec_context.compute()?)
        } else {
            Err(UsageError::InvalidExecutionContextHandle.into())
        }
    }

    fn get_output<'b>(
        &mut self,
        exec_context_id: GraphExecutionContext,
        index: u32,
        out_buffer: &GuestPtr<'_, u8>,
        out_buffer_max_size: u32,
    ) -> Result<u32> {
        let mut destination = out_buffer.as_array(out_buffer_max_size).as_slice_mut()?;
        if let Some(exec_context) = self.ctx.borrow_mut().executions.get_mut(exec_context_id) {
            Ok(exec_context.get_output(index, &mut destination)?)
        } else {
            Err(UsageError::InvalidGraphHandle.into())
        }
    }
}
