//! Implements the wasi-nn API.

use std::collections::HashMap;
use crate::api::{Backend, BackendError, BackendExecutionContext, BackendGraph};
use crate::witx::types::{ExecutionTarget, GraphBuilderArray, Tensor, TensorType};
use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::Number;

#[derive(Default)]
pub(crate) struct KServeBackend();

unsafe impl Send for KServeBackend {}

unsafe impl Sync for KServeBackend {}

impl Backend for KServeBackend {
    fn name(&self) -> &str {
        "KServe"
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

struct KServeGraph();

unsafe impl Send for KServeGraph {}

unsafe impl Sync for KServeGraph {}

impl BackendGraph for KServeGraph {
    fn init_execution_context(&mut self) -> Result<Box<dyn BackendExecutionContext>, BackendError> {
        return Err(BackendError::UnsupportedOperation("init_execution_context"));
    }
}

struct KServeExecutionContext(Arc<openvino::CNNNetwork>, openvino::InferRequest);

impl BackendExecutionContext for KServeExecutionContext {
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

struct KServeClient {
    server_url: String,

}

impl KServeClient {
    //Bulk of the logic will be here.

    fn build_inference_url(model_name: String) -> String {
        return format!("/v2/models/{}/infer", model_name);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct KServeTensorParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_data_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_data: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeTensorMetadata {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    shape: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    datatype: Option<KServeDatatype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<KServeTensorParameters>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeBinaryInferenceRequest {
    model_name: String,
    inputs: Vec<KServeTensorMetadata>,
    outputs: Vec<KServeTensorMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeBinaryInferenceResult {
    outputs: Vec<KServeTensorMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeModelConfig {
    //For some weird reason the json here expects name, instead of model_name like in inference request.
    pub name: String,
    pub backend: String,
    pub inputs: Vec<KServeTensorMetadata>,
    pub outputs: Vec<KServeTensorMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KServeRepositoryLoadRequest {
    parameters: KServeParameters,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KServeRepositoryLoadErrorResponse {
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KServeParameterValue {
    Bool(bool),
    Number(Number),
    String(String),
    Config(KServeModelConfig),
}

type KServeParameters = HashMap<String, KServeParameterValue>;

#[derive(Debug, Serialize, Deserialize)]
pub enum KServeDatatype {
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

#[test]
fn test_kserve_model_config_deserialization() {
    let json = r#"
    {
          "name": "mymodel",
          "backend": "onnxruntime",
          "inputs": [{
              "name": "INPUT0",
              "datatype": "FP32",
              "shape": [ 1 ]
            }
          ],
          "outputs": [{
              "name": "OUTPUT0",
              "datatype": "FP32",
              "shape": [ 1 ]
          }]
    }
    "#;

    let output = serde_json::from_str::<KServeModelConfig>(json).unwrap();
}

#[test]
fn test_deserialization() {
    let json = r#"
    {
      "parameters": {
        "a" : true,
        "b" : 1.0,
        "c" : 1,
        "d" : "efg",
        "config": {
          "name": "mymodel",
          "backend": "onnxruntime",
          "inputs": [{
              "name": "INPUT0",
              "datatype": "FP32",
              "shape": [ 1 ]
            }
          ],
          "outputs": [{
              "name": "OUTPUT0",
              "datatype": "FP32",
              "shape": [ 1 ]
          }]
        }
      }
    }
    "#;

    let output = serde_json::from_str::<KServeRepositoryLoadRequest>(json).unwrap();
}