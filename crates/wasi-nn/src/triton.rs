//! Implements the wasi-nn API.

use crate::api::{Backend, BackendError, BackendExecutionContext, BackendGraph};
use crate::witx::types::{ExecutionTarget, GraphBuilderArray, Tensor, TensorType};
use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub(crate) struct TritonBackend();

unsafe impl Send for TritonBackend {}

unsafe impl Sync for TritonBackend {}

impl Backend for TritonBackend {
    fn name(&self) -> &str {
        "triton"
    }

    fn load(
        &mut self,
        builders: &GraphBuilderArray<'_>,
        target: ExecutionTarget,
    ) -> Result<Box<dyn BackendGraph>, BackendError> {
        return Err(BackendError::UnsupportedOperation("load"));
    }

    fn load_from_bytes(
        &mut self,
        model_bytes: &Vec<Vec<u8>>,
        target: ExecutionTarget,
    ) -> Result<Box<dyn BackendGraph>, BackendError> {
        return Err(BackendError::UnsupportedOperation("load_from_bytes"));
    }
}

struct TritonGraph();

unsafe impl Send for TritonGraph {}

unsafe impl Sync for TritonGraph {}

impl BackendGraph for TritonGraph {
    fn init_execution_context(&mut self) -> Result<Box<dyn BackendExecutionContext>, BackendError> {
        return Err(BackendError::UnsupportedOperation("init_execution_context"));
    }
}

struct TritonExecutionContext(Arc<openvino::CNNNetwork>, openvino::InferRequest);

impl BackendExecutionContext for TritonExecutionContext {
    fn set_input(&mut self, index: u32, tensor: &Tensor<'_>) -> Result<(), BackendError> {
        return Err(BackendError::UnsupportedOperation("init_execution_context"));
    }

    fn compute(&mut self) -> Result<(), BackendError> {
        self.1.infer()?;
        Ok(())
    }

    fn get_output(&mut self, index: u32, destination: &mut [u8]) -> Result<u32, BackendError> {
        let output_name = self.0.get_output_name(index as usize)?;
        let blob = self.1.get_blob(&output_name)?;
        let blob_size = blob.byte_len()?;
        if blob_size > destination.len() {
            return Err(BackendError::NotEnoughMemory(blob_size));
        }

        // Copy the tensor data into the destination buffer.
        destination[..blob_size].copy_from_slice(blob.buffer()?);
        Ok(blob_size as u32)
    }
}

/// Return the execution target string expected by OpenVINO from the
/// `ExecutionTarget` enum provided by wasi-nn.
fn map_execution_target_to_string(target: ExecutionTarget) -> &'static str {
    match target {
        ExecutionTarget::Cpu => "CPU",
        ExecutionTarget::Gpu => "GPU",
        ExecutionTarget::Tpu => "TPU",
    }
}

struct TritonClient {
    server_url: String,

}

impl TritonClient {
    //Bulk of the logic will be here.

    fn build_inference_url(model_name: String) -> String {
        return format!("/v2/models/{}/infer", model_name)
    }


}


#[derive(Serialize, Deserialize)]
struct TritonTensorParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_data_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_data: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct TritonTensorMetadata {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    shape: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    datatype: Option<TritonDatatype>,
    parameters: TritonTensorParameters,
}

#[derive(Serialize, Deserialize)]
struct TritonBinaryInferenceRequest {
    model_name: String,
    inputs: Vec<TritonTensorMetadata>,
    outputs: Vec<TritonTensorMetadata>,
}

#[derive(Serialize, Deserialize)]
struct TritonBinaryInferenceResult {
    outputs: Vec<TritonTensorMetadata>,
}

#[derive(Serialize, Deserialize)]
pub enum TritonDatatype {
    BOOL,
    UINT8,
    UINT16,
    UINT32,
    UINT64,
    INT8,
    INT16,
    INT32,
    INT64,
    FP16,
    FP32,
    FP64,
    BYTES,
    BF16,
}