//! Implements a `wasi-nn` [`BackendInner`] using WinML.

use super::{BackendError, BackendExecutionContext, BackendFromDir, BackendGraph, BackendInner};
use crate::wit::types::{ExecutionTarget, GraphEncoding, Tensor};
use crate::{ExecutionContext, Graph};
use std::{fs::File, io::Read, mem::size_of, path::Path};
use windows::core::{ComInterface, HSTRING};
use windows::Storage::Streams::{
    DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference,
};
use windows::AI::MachineLearning::{
    LearningModel, LearningModelBinding, LearningModelDevice, LearningModelDeviceKind,
    LearningModelEvaluationResult, LearningModelSession, TensorFeatureDescriptor, TensorFloat,
};

#[derive(Default)]
pub struct WinMLBackend();

impl BackendInner for WinMLBackend {
    fn encoding(&self) -> GraphEncoding {
        GraphEncoding::Onnx
    }

    fn load(&mut self, builders: &[&[u8]], target: ExecutionTarget) -> Result<Graph, BackendError> {
        if builders.len() != 1 {
            return Err(BackendError::InvalidNumberOfBuilders(1, builders.len()).into());
        }

        let model_stream = InMemoryRandomAccessStream::new()?;
        let model_writer = DataWriter::CreateDataWriter(&model_stream)?;
        model_writer.WriteBytes(&builders[0])?;
        model_writer.StoreAsync()?;
        model_writer.FlushAsync()?;
        let model = LearningModel::LoadFromStream(&RandomAccessStreamReference::CreateFromStream(
            &model_stream,
        )?)?;
        let device_kind = match target {
            ExecutionTarget::Cpu => LearningModelDeviceKind::Cpu,
            ExecutionTarget::Gpu => LearningModelDeviceKind::DirectX,
            ExecutionTarget::Tpu => unimplemented!(),
        };
        let graph = WinMLGraph { model, device_kind };

        let box_: Box<dyn BackendGraph> = Box::new(graph);
        Ok(box_.into())
    }

    fn as_dir_loadable(&mut self) -> Option<&mut dyn BackendFromDir> {
        Some(self)
    }
}

impl BackendFromDir for WinMLBackend {
    fn load_from_dir(
        &mut self,
        path: &Path,
        target: ExecutionTarget,
    ) -> Result<Graph, BackendError> {
        let model = read(&path.join("model.onnx"))?;
        self.load(&[&model], target)
    }
}

struct WinMLGraph {
    model: LearningModel,
    device_kind: LearningModelDeviceKind,
}

unsafe impl Send for WinMLGraph {}
unsafe impl Sync for WinMLGraph {}

impl BackendGraph for WinMLGraph {
    fn init_execution_context(&self) -> Result<ExecutionContext, BackendError> {
        let device = LearningModelDevice::Create(self.device_kind.clone())?;
        let session = LearningModelSession::CreateFromModelOnDevice(&self.model, &device)?;
        let box_: Box<dyn BackendExecutionContext> = Box::new(WinMLExecutionContext::new(session));
        Ok(box_.into())
    }
}

struct WinMLExecutionContext {
    session: LearningModelSession,
    binding: LearningModelBinding,
    result: Option<LearningModelEvaluationResult>,
}

impl WinMLExecutionContext {
    fn new(session: LearningModelSession) -> Self {
        Self {
            binding: LearningModelBinding::CreateFromSession(&session).unwrap(),
            session,
            result: None,
        }
    }
}

impl BackendExecutionContext for WinMLExecutionContext {
    fn set_input(&mut self, index: u32, tensor: &Tensor) -> Result<(), BackendError> {
        // TODO: Support other tensor types. Only FP32 is supported right now.
        match tensor.tensor_type {
            crate::wit::types::TensorType::Fp32 => {}
            _ => unimplemented!(),
        }

        let input = self.session.Model()?.InputFeatures()?.GetAt(index)?;
        unsafe {
            let data = std::slice::from_raw_parts(
                tensor.data.as_ptr() as *const f32,
                tensor.data.len() / 4,
            );

            self.binding.Bind(
                &input.Name()?,
                &TensorFloat::CreateFromArray(
                    &input.cast::<TensorFeatureDescriptor>()?.Shape()?,
                    data,
                )?,
            )?;
        }
        Ok(())
    }

    fn compute(&mut self) -> Result<(), BackendError> {
        self.result = Some(self.session.Evaluate(&self.binding, &HSTRING::new())?);
        Ok(())
    }

    fn get_output(&mut self, index: u32, destination: &mut [u8]) -> Result<u32, BackendError> {
        if self.result.is_none() {
            return Err(BackendError::BackendAccess(anyhow::Error::msg(
                "Output is not ready.",
            )));
        }
        let output_name = self.session.Model()?.OutputFeatures()?.GetAt(index)?;
        let output_name_hstring = output_name.Name()?;

        let vector_view = self
            .result
            .as_ref()
            .unwrap()
            .Outputs()?
            .Lookup(&output_name_hstring)?
            .cast::<TensorFloat>()?
            .GetAsVectorView()?;
        let output: Vec<f32> = vector_view.into_iter().collect();
        let len_to_copy = output.len() * size_of::<f32>();
        unsafe {
            destination[..len_to_copy].copy_from_slice(std::slice::from_raw_parts(
                output.as_ptr() as *const u8,
                len_to_copy,
            ));
        }

        Ok(len_to_copy as u32)
    }
}

/// Read a file into a byte vector.
fn read(path: &Path) -> anyhow::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = vec![];
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

impl From<windows::core::Error> for BackendError {
    fn from(e: windows::core::Error) -> Self {
        BackendError::BackendAccess(anyhow::Error::new(e))
    }
}
