//! Implements a `wasi-nn` [`BackendInner`] using WinML.
//!
//! Note that the [docs.rs] documentation for the `windows` crate does have the
//! right features turned on to read about the functions used; see Microsoft's
//! private documentation instead: [microsoft.github.io/windows-docs-rs].
//!
//! [docs.rs]: https://docs.rs/windows
//! [microsoft.github.io/windows-docs-rs]: https://microsoft.github.io/windows-docs-rs/doc/windows/AI/MachineLearning

use crate::backend::{
    BackendError, BackendExecutionContext, BackendFromDir, BackendGraph, BackendInner, Id,
};
use crate::wit::{ExecutionTarget, GraphEncoding, Tensor, TensorType};
use crate::{ExecutionContext, Graph};
use std::{fs::File, io::Read, mem::size_of, path::Path};
use windows::core::{ComInterface, Error, IInspectable, HSTRING};
use windows::Foundation::Collections::IVectorView;
use windows::Storage::Streams::{
    DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference,
};
use windows::AI::MachineLearning::{
    ILearningModelFeatureDescriptor, LearningModel, LearningModelBinding, LearningModelDevice,
    LearningModelDeviceKind, LearningModelEvaluationResult, LearningModelSession,
    TensorFeatureDescriptor, TensorFloat, TensorFloat16Bit, TensorInt64Bit, TensorKind,
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

impl WinMLExecutionContext {
    /// Helper function for finding the internal index of a tensor by [`Id`].
    fn find(
        &self,
        id: Id,
        list: &IVectorView<ILearningModelFeatureDescriptor>,
    ) -> Result<u32, BackendError> {
        let index = match id {
            Id::Index(i) => {
                if i < list.Size()? {
                    i
                } else {
                    return Err(BackendError::BackendAccess(anyhow::anyhow!(
                        "incorrect tensor index: {i} >= {}",
                        list.Size()?
                    )));
                }
            }
            Id::Name(name) => list
                .into_iter()
                .position(|d| d.Name().unwrap() == name)
                .ok_or_else(|| {
                    BackendError::BackendAccess(anyhow::anyhow!("unknown tensor name: {name}"))
                })? as u32,
        };
        Ok(index)
    }
}

impl BackendExecutionContext for WinMLExecutionContext {
    fn set_input(&mut self, id: Id, tensor: &Tensor) -> Result<(), BackendError> {
        // TODO: Clear previous bindings when needed.

        let input_features = self.session.Model()?.InputFeatures()?;
        let index = self.find(id, &input_features)?;
        let input = input_features.GetAt(index)?;

        let inpsectable =
            to_inspectable(tensor, input.cast::<TensorFeatureDescriptor>()?.Shape()?)?;
        self.binding.Bind(&input.Name()?, &inpsectable)?;

        Ok(())
    }

    fn compute(&mut self) -> Result<(), BackendError> {
        self.result = Some(self.session.Evaluate(&self.binding, &HSTRING::new())?);
        Ok(())
    }

    fn get_output(&mut self, id: Id) -> Result<Tensor, BackendError> {
        if let Some(result) = &self.result {
            let output_features = self.session.Model()?.OutputFeatures()?;
            let index = self.find(id, &output_features)?;
            let output_feature = output_features.GetAt(index)?;
            let tensor_kind = match output_feature.Kind()? {
                windows::AI::MachineLearning::LearningModelFeatureKind::Tensor => output_feature
                    .cast::<TensorFeatureDescriptor>()?
                    .TensorKind()?,
                _ => unimplemented!(),
            };
            // TODO: this only handles FP16, FP32 and I64!
            let tensor = to_tensor(
                result.Outputs()?.Lookup(&output_feature.Name()?)?,
                tensor_kind,
            );
            tensor
        } else {
            return Err(BackendError::BackendAccess(anyhow::Error::msg(
                "Output is not ready.",
            )));
        }
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

fn dimensions_as_u32(dimensions: &IVectorView<i64>) -> Result<Vec<u32>, BackendError> {
    dimensions
        .into_iter()
        .map(|d| if d == -1 { Ok(1) } else { convert_i64(d) })
        .collect()
}

fn convert_i64(i: i64) -> Result<u32, BackendError> {
    u32::try_from(i).map_err(|d| -> BackendError {
        anyhow::anyhow!("unable to convert dimension to u32: {d}").into()
    })
}

// Convert from wasi-nn tensor to WinML tensor.
fn to_inspectable(tensor: &Tensor, shape: IVectorView<i64>) -> Result<IInspectable, Error> {
    match tensor.ty {
        crate::wit::types::TensorType::Fp16 => unsafe {
            let data = std::slice::from_raw_parts(
                tensor.data.as_ptr() as *const f32,
                tensor.data.len() / size_of::<f32>(),
            );
            TensorFloat16Bit::CreateFromArray(&shape, data)?.cast::<IInspectable>()
        },
        crate::wit::types::TensorType::Fp32 => unsafe {
            let data = std::slice::from_raw_parts(
                tensor.data.as_ptr() as *const f32,
                tensor.data.len() / size_of::<f32>(),
            );
            TensorFloat::CreateFromArray(&shape, data)?.cast::<IInspectable>()
        },
        crate::wit::types::TensorType::I64 => unsafe {
            let data = std::slice::from_raw_parts(
                tensor.data.as_ptr() as *const i64,
                tensor.data.len() / size_of::<i64>(),
            );
            TensorInt64Bit::CreateFromArray(&shape, data)?.cast::<IInspectable>()
        },
        _ => unimplemented!(),
    }
}

// Convert from WinML tensor to wasi-nn tensor.
fn to_tensor(inspectable: IInspectable, tensor_kind: TensorKind) -> Result<Tensor, BackendError> {
    let tensor = match tensor_kind {
        TensorKind::Float16 => {
            let output_tensor = inspectable.cast::<TensorFloat16Bit>()?;
            let dimensions = dimensions_as_u32(&output_tensor.Shape()?)?;
            let view = output_tensor.GetAsVectorView()?;
            // TODO: Move to f16 when it's available in stable.
            let mut data = Vec::with_capacity(view.Size()? as usize * size_of::<f32>());
            for f in view.into_iter() {
                data.extend(f.to_le_bytes());
            }
            Tensor {
                ty: TensorType::Fp16,
                dimensions,
                data,
            }
        }
        TensorKind::Float => {
            let output_tensor = inspectable.cast::<TensorFloat>()?;
            let dimensions = dimensions_as_u32(&output_tensor.Shape()?)?;
            let view = output_tensor.GetAsVectorView()?;
            let mut data = Vec::with_capacity(view.Size()? as usize * size_of::<f32>());
            for f in view.into_iter() {
                data.extend(f.to_le_bytes());
            }
            Tensor {
                ty: TensorType::Fp32,
                dimensions,
                data,
            }
        }
        TensorKind::Int64 => {
            let output_tensor = inspectable.cast::<TensorInt64Bit>()?;
            let dimensions = dimensions_as_u32(&output_tensor.Shape()?)?;
            let view = output_tensor.GetAsVectorView()?;
            let mut data = Vec::with_capacity(view.Size()? as usize * size_of::<i64>());
            for f in view.into_iter() {
                data.extend(f.to_le_bytes());
            }
            Tensor {
                ty: TensorType::I64,
                dimensions,
                data,
            }
        }
        _ => unimplemented!(),
    };
    Ok(tensor)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests for different data types. Convert from wasi-nn tensor to WinML tensor and back.
    #[test]
    fn fp16() {
        let data = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let mut buffer = Vec::with_capacity(data.len() * size_of::<f32>());
        for f in &data {
            buffer.extend(f.to_ne_bytes());
        }
        let buffer_copy = buffer.clone();
        let tensor = Tensor {
            ty: TensorType::Fp16,
            dimensions: vec![2, 3],
            data: buffer_copy,
        };
        let shape = IVectorView::<i64>::try_from(vec![2i64, 3]).unwrap();
        let inspectable = to_inspectable(&tensor, shape);
        assert!(inspectable.is_ok());
        let winml_tensor = inspectable
            .as_ref()
            .unwrap()
            .cast::<TensorFloat16Bit>()
            .unwrap();
        let view = winml_tensor.GetAsVectorView().unwrap();
        assert_eq!(view.into_iter().collect::<Vec<f32>>(), data);
        // Convert back.
        let t = to_tensor(inspectable.unwrap(), TensorKind::Float16);
        assert!(t.as_ref().is_ok());
        let t_ref = t.as_ref();
        assert_eq!(t_ref.unwrap().data, buffer);
        assert_eq!(t_ref.unwrap().dimensions, tensor.dimensions);
        assert_eq!(t_ref.unwrap().ty, TensorType::Fp16);
    }

    #[test]
    fn fp32() {
        let data = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let mut buffer = Vec::with_capacity(data.len() * size_of::<f32>());
        for f in &data {
            buffer.extend(f.to_ne_bytes());
        }
        let buffer_copy = buffer.clone();
        let tensor = Tensor {
            ty: TensorType::Fp32,
            dimensions: vec![2, 3],
            data: buffer_copy,
        };
        let shape = IVectorView::<i64>::try_from(vec![2i64, 3]).unwrap();
        let inspectable = to_inspectable(&tensor, shape);
        assert!(inspectable.is_ok());
        let winml_tensor = inspectable.as_ref().unwrap().cast::<TensorFloat>().unwrap();
        let view = winml_tensor.GetAsVectorView().unwrap();
        assert_eq!(view.into_iter().collect::<Vec<f32>>(), data);
        // Convert back.
        let t = to_tensor(inspectable.unwrap(), TensorKind::Float);
        assert!(t.as_ref().is_ok());
        let t_ref = t.as_ref();
        assert_eq!(t_ref.unwrap().data, buffer);
        assert_eq!(t_ref.unwrap().dimensions, tensor.dimensions);
        assert_eq!(t_ref.unwrap().ty, TensorType::Fp32);
    }

    #[test]
    fn i64() {
        let data = vec![6i64, 5, 4, 3, 2, 1];
        let mut buffer = Vec::with_capacity(data.len() * size_of::<i64>());
        for f in &data {
            buffer.extend(f.to_ne_bytes());
        }
        let buffer_copy = buffer.clone();
        let tensor = Tensor {
            ty: TensorType::I64,
            dimensions: vec![1, 6],
            data: buffer_copy,
        };
        let shape = IVectorView::<i64>::try_from(vec![1i64, 6]).unwrap();
        let inspectable = to_inspectable(&tensor, shape);
        assert!(inspectable.is_ok());
        let winml_tensor = inspectable
            .as_ref()
            .unwrap()
            .cast::<TensorInt64Bit>()
            .unwrap();
        let view = winml_tensor.GetAsVectorView().unwrap();
        assert_eq!(view.into_iter().collect::<Vec<i64>>(), data);
        // Convert back.
        let t = to_tensor(inspectable.unwrap(), TensorKind::Int64);
        assert!(t.as_ref().is_ok());
        let t_ref = t.as_ref();
        assert_eq!(t_ref.unwrap().data, buffer);
        assert_eq!(t_ref.unwrap().dimensions, tensor.dimensions);
        assert_eq!(t_ref.unwrap().ty, TensorType::I64);
    }
}
