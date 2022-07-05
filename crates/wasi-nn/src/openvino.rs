//! Implements the wasi-nn API.
use crate::api::{Backend, BackendError, BackendExecutionContext, BackendGraph};
use crate::witx::types::{ExecutionTarget, GraphBuilderArray, Tensor, TensorType};
use openvino::{InferenceError, Layout, Precision, SetupError, TensorDesc};
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct OpenvinoBackend(Option<openvino::Core>);

impl Backend for OpenvinoBackend {
    fn name(&self) -> &str {
        "openvino"
    }

    fn load(
        &mut self,
        builders: &GraphBuilderArray<'_>,
        target: ExecutionTarget,
    ) -> Result<Box<dyn BackendGraph>, BackendError> {
        if builders.len() != 2 {
            return Err(BackendError::InvalidNumberOfBuilders(2, builders.len()).into());
        }

        // Construct the context if none is present; this is done lazily (i.e.
        // upon actually loading a model) because it may fail to find and load
        // the OpenVINO libraries. The laziness limits the extent of the error
        // only to wasi-nn users, not all WASI users.
        if self.0.is_none() {
            self.0.replace(openvino::Core::new(None)?);
        }

        // Read the guest array.
        let builders = builders.as_ptr();
        let xml = builders.read()?.as_slice()?;
        let weights = builders.add(1)?.read()?.as_slice()?;

        // Construct OpenVINO graph structures: `cnn_network` contains the graph
        // structure, `exec_network` can perform inference.
        let core = self
            .0
            .as_mut()
            .expect("openvino::Core was previously constructed");
        let mut cnn_network = core.read_network_from_buffer(&xml, &weights)?;

        // TODO this is a temporary workaround. We need a more eligant way to specify the layout in the long run.
        // However, without this newer versions of OpenVINO will fail due to parameter mismatch.
        for i in 0..cnn_network.get_inputs_len()? {
            let name = cnn_network.get_input_name(i)?;
            cnn_network.set_input_layout(&name, Layout::NHWC)?;
        }

        let exec_network =
            core.load_network(&cnn_network, map_execution_target_to_string(target))?;

        Ok(Box::new(OpenvinoGraph(Arc::new(cnn_network), exec_network)))
    }
}

struct OpenvinoGraph(Arc<openvino::CNNNetwork>, openvino::ExecutableNetwork);

impl BackendGraph for OpenvinoGraph {
    fn init_execution_context(&mut self) -> Result<Box<dyn BackendExecutionContext>, BackendError> {
        let infer_request = self.1.create_infer_request()?;
        Ok(Box::new(OpenvinoExecutionContext(
            self.0.clone(),
            infer_request,
        )))
    }
}

struct OpenvinoExecutionContext(Arc<openvino::CNNNetwork>, openvino::InferRequest);

impl BackendExecutionContext for OpenvinoExecutionContext {
    fn set_input(&mut self, index: u32, tensor: &Tensor<'_>) -> Result<(), BackendError> {
        let input_name = self.0.get_input_name(index as usize)?;

        // Construct the blob structure.
        let dimensions = tensor
            .dimensions
            .as_slice()?
            .iter()
            .map(|d| *d as usize)
            .collect::<Vec<_>>();
        let precision = map_tensor_type_to_precision(tensor.type_);

        // TODO There must be some good way to discover the layout here; this
        // should not have to default to NHWC.
        let desc = TensorDesc::new(Layout::NHWC, &dimensions, precision);
        let data = tensor.data.as_slice()?;
        let blob = openvino::Blob::new(&desc, &data)?;

        // Actually assign the blob to the request.
        self.1.set_blob(&input_name, &blob)?;
        Ok(())
    }

    fn compute(&mut self) -> Result<(), BackendError> {
        self.1.infer()?;
        Ok(())
    }

    fn get_output(&mut self, index: u32, destination: &mut [u8]) -> Result<u32, BackendError> {
        let output_name = self.0.get_output_name(index as usize)?;
        let mut blob = self.1.get_blob(&output_name)?;
        let blob_size = blob.byte_len()?;
        if blob_size > destination.len() {
            return Err(BackendError::NotEnoughMemory(blob_size));
        }

        // Copy the tensor data into the destination buffer.
        destination[..blob_size].copy_from_slice(blob.buffer()?);
        Ok(blob_size as u32)
    }
}

impl From<InferenceError> for BackendError {
    fn from(e: InferenceError) -> Self {
        BackendError::BackendAccess(anyhow::Error::new(e))
    }
}

impl From<SetupError> for BackendError {
    fn from(e: SetupError) -> Self {
        BackendError::BackendAccess(anyhow::Error::new(e))
    }
}

/// Return the execution target string expected by OpenVINO from the
/// `ExecutionTarget` enum provided by wasi-nn.
fn map_execution_target_to_string(target: ExecutionTarget) -> &'static str {
    match target {
        ExecutionTarget::Cpu => "CPU",
        ExecutionTarget::Gpu => "GPU",
        ExecutionTarget::Tpu => unimplemented!("OpenVINO does not support TPU execution targets"),
    }
}

/// Return OpenVINO's precision type for the `TensorType` enum provided by
/// wasi-nn.
fn map_tensor_type_to_precision(tensor_type: TensorType) -> openvino::Precision {
    match tensor_type {
        TensorType::F16 => Precision::FP16,
        TensorType::F32 => Precision::FP32,
        TensorType::U8 => Precision::U8,
        TensorType::I32 => Precision::I32,
    }
}
