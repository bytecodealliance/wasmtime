//! Implements the wasi-nn API.
use crate::ctx::{ExecutionContext, WasiNnResult as Result};
use crate::witx::types::{
    ExecutionTarget, Graph, GraphBuilderArray, GraphEncoding, GraphExecutionContext, Tensor,
    TensorType,
};
use crate::witx::wasi_ephemeral_nn::WasiEphemeralNn;
use crate::WasiNnCtx;
use openvino::{Layout, Precision, TensorDesc};
use thiserror::Error;
use wiggle::GuestPtr;

#[derive(Debug, Error)]
pub enum UsageError {
    #[error("Only OpenVINO's IR is currently supported, passed encoding: {0}")]
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
        &self,
        builders: &GraphBuilderArray<'_>,
        encoding: GraphEncoding,
        target: ExecutionTarget,
    ) -> Result<Graph> {
        if encoding != GraphEncoding::Openvino {
            return Err(UsageError::InvalidEncoding(encoding).into());
        }
        if builders.len() != 2 {
            return Err(UsageError::InvalidNumberOfBuilders(builders.len()).into());
        }
        let builders = builders.as_ptr();
        let xml = builders.read()?.as_slice()?;
        let weights = builders.add(1)?.read()?.as_slice()?;
        let graph = self
            .ctx
            .borrow_mut()
            .core
            .read_network_from_buffer(&xml, &weights)?;
        let executable_graph = self
            .ctx
            .borrow_mut()
            .core
            .load_network(&graph, map_execution_target_to_string(target))?;
        let id = self
            .ctx
            .borrow_mut()
            .graphs
            .insert((graph, executable_graph));
        Ok(id)
    }

    fn init_execution_context(&self, graph: Graph) -> Result<GraphExecutionContext> {
        let request =
            if let Some((_, executable_graph)) = self.ctx.borrow_mut().graphs.get_mut(graph) {
                executable_graph.create_infer_request()?
            } else {
                return Err(UsageError::InvalidGraphHandle.into());
            };

        let execution_context = ExecutionContext::new(graph, request);
        let handle = self.ctx.borrow_mut().executions.insert(execution_context);
        Ok(handle)
    }

    fn set_input<'b>(
        &self,
        context: GraphExecutionContext,
        index: u32,
        tensor: &Tensor<'b>,
    ) -> Result<()> {
        let graph = if let Some(execution) = self.ctx.borrow_mut().executions.get_mut(context) {
            execution.graph
        } else {
            return Err(UsageError::InvalidExecutionContextHandle.into());
        };

        let input_name = if let Some((graph, _)) = self.ctx.borrow().graphs.get(graph) {
            graph.get_input_name(index as usize)?
        } else {
            unreachable!("It should be impossible to attempt to access an execution's graph and for that graph not to exist--this is a bug.")
        };

        // Construct the blob structure.
        let dimensions = tensor
            .dimensions
            .as_slice()?
            .iter()
            .map(|d| *d as u64)
            .collect::<Vec<_>>();
        let precision = match tensor.type_ {
            TensorType::F16 => Precision::FP16,
            TensorType::F32 => Precision::FP32,
            TensorType::U8 => Precision::U8,
            TensorType::I32 => Precision::I32,
        };
        // TODO There must be some good way to discover the layout here; this should not have to default to NHWC.
        let desc = TensorDesc::new(Layout::NHWC, &dimensions, precision);
        let data = tensor.data.as_slice()?;
        let blob = openvino::Blob::new(desc, &data)?;

        // Actually assign the blob to the request (TODO avoid duplication with the borrow above).
        if let Some(execution) = self.ctx.borrow_mut().executions.get_mut(context) {
            execution.request.set_blob(&input_name, blob)?;
        } else {
            return Err(UsageError::InvalidExecutionContextHandle.into());
        }

        Ok(())
    }

    fn compute(&self, context: GraphExecutionContext) -> Result<()> {
        if let Some(execution) = self.ctx.borrow_mut().executions.get_mut(context) {
            Ok(execution.request.infer()?)
        } else {
            return Err(UsageError::InvalidExecutionContextHandle.into());
        }
    }

    fn get_output<'b>(
        &self,
        context: GraphExecutionContext,
        index: u32,
        out_buffer: &GuestPtr<'_, u8>,
        out_buffer_max_size: u32,
    ) -> Result<u32> {
        let graph = if let Some(execution) = self.ctx.borrow_mut().executions.get_mut(context) {
            execution.graph
        } else {
            return Err(UsageError::InvalidExecutionContextHandle.into());
        };

        let output_name = if let Some((graph, _)) = self.ctx.borrow().graphs.get(graph) {
            graph.get_output_name(index as usize)?
        } else {
            unreachable!("It should be impossible to attempt to access an execution's graph and for that graph not to exist--this is a bug.")
        };

        // Retrieve the tensor data.
        let (mut blob, blob_size) =
            if let Some(execution) = self.ctx.borrow_mut().executions.get_mut(context) {
                let mut blob = execution.request.get_blob(&output_name)?; // TODO shouldn't need to be mut
                let blob_size = blob.byte_len()? as u32;
                if blob_size > out_buffer_max_size {
                    return Err(UsageError::NotEnoughMemory(blob_size).into());
                }
                (blob, blob_size)
            } else {
                return Err(UsageError::InvalidExecutionContextHandle.into());
            };

        // Copy the tensor data over to the `out_buffer`.
        let mut out_slice = out_buffer.as_array(out_buffer_max_size).as_slice()?;
        (&mut out_slice[..blob_size as usize]).copy_from_slice(blob.buffer()?);

        Ok(blob_size)
    }
}

/// Return the execution target string expected by OpenVINO from the `ExecutionTarget` enum provided
/// by wasi-nn.
fn map_execution_target_to_string(target: ExecutionTarget) -> &'static str {
    match target {
        ExecutionTarget::Cpu => "CPU",
        ExecutionTarget::Gpu => "GPU",
        ExecutionTarget::Tpu => unimplemented!("OpenVINO does not support TPU execution targets"),
    }
}
