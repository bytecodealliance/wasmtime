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
use windows::Win32::Graphics::DXCore::{
    DXCoreCreateAdapterFactory, IDXCoreAdapter, IDXCoreAdapterFactory, IDXCoreAdapterList,
    DXCORE_ADAPTER_ATTRIBUTE_D3D12_CORE_COMPUTE, DXCORE_ADAPTER_ATTRIBUTE_D3D12_GRAPHICS,
};
use windows::Win32::Graphics::{
    Direct3D::D3D_FEATURE_LEVEL_1_0_CORE,
    Direct3D12::{
        D3D12CreateDevice, ID3D12CommandQueue, ID3D12Device, D3D12_COMMAND_LIST_TYPE_COMPUTE,
        D3D12_COMMAND_QUEUE_DESC, D3D12_COMMAND_QUEUE_FLAG_NONE,
    },
};
use windows::Win32::System::WinRT::ML::ILearningModelDeviceFactoryNative;
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
        let device = match target {
            ExecutionTarget::Cpu => LearningModelDevice::Create(LearningModelDeviceKind::Cpu),
            ExecutionTarget::Gpu => LearningModelDevice::Create(LearningModelDeviceKind::DirectX),
            ExecutionTarget::Tpu => unsafe {
                // Enumerate adapters with DXCore APIs so MCDM (Microsoft Compute Driver Model) devices can be found.
                let dx_adapter_factory: IDXCoreAdapterFactory = DXCoreCreateAdapterFactory()?;
                let adapter_list =
                    dx_adapter_factory.CreateAdapterList::<IDXCoreAdapterList>(&[
                        DXCORE_ADAPTER_ATTRIBUTE_D3D12_CORE_COMPUTE,
                    ])?;
                let mut selected_device: Option<IDXCoreAdapter> = None;
                for i in 0..adapter_list.GetAdapterCount() {
                    let adapter = adapter_list.GetAdapter::<IDXCoreAdapter>(i)?;
                    // Select a compute only device. DXCORE_ADAPTER_ATTRIBUTE_D3D12_GENERIC_ML looks more suitable here, but it's defined in DirectX headers.
                    if adapter.IsAttributeSupported(&DXCORE_ADAPTER_ATTRIBUTE_D3D12_CORE_COMPUTE)
                        && !adapter.IsAttributeSupported(&DXCORE_ADAPTER_ATTRIBUTE_D3D12_GRAPHICS)
                    {
                        selected_device = Some(adapter);
                        break;
                    }
                }
                if selected_device.is_none() {
                    return Err(BackendError::BackendAccess(anyhow::Error::msg(
                        "NPU is not available on this device.",
                    )));
                }

                let mut d3d12_device: Option<ID3D12Device> = None;
                D3D12CreateDevice(
                    &selected_device.unwrap(),
                    D3D_FEATURE_LEVEL_1_0_CORE,
                    &mut d3d12_device,
                )?;
                if d3d12_device.is_none() {
                    return Err(BackendError::BackendAccess(anyhow::Error::msg(
                        "Failed to create D3D12 device.",
                    )));
                }
                let d3d12_command_queue_desc: D3D12_COMMAND_QUEUE_DESC = D3D12_COMMAND_QUEUE_DESC {
                    Type: D3D12_COMMAND_LIST_TYPE_COMPUTE,
                    Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                    NodeMask: 0,
                    Priority: 0,
                };
                let d3d12_command_queue = d3d12_device
                    .unwrap()
                    .CreateCommandQueue::<ID3D12CommandQueue>(&d3d12_command_queue_desc)?;
                let factory = windows::core::factory::<
                    LearningModelDevice,
                    ILearningModelDeviceFactoryNative,
                >()?;
                factory
                    .CreateFromD3D12CommandQueue(&d3d12_command_queue)?
                    .cast::<LearningModelDevice>()
            },
        };
        let graph = WinMLGraph {
            model,
            device: device?,
        };

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
    device: LearningModelDevice,
}

unsafe impl Send for WinMLGraph {}
unsafe impl Sync for WinMLGraph {}

impl BackendGraph for WinMLGraph {
    fn init_execution_context(&self) -> Result<ExecutionContext, BackendError> {
        let session =
            LearningModelSession::CreateFromModelOnDevice(&self.model, &self.device).unwrap();
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

        let inspectable = to_inspectable(tensor)?;
        self.binding.Bind(&input.Name()?, &inspectable)?;

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
                _ => unimplemented!(
                    "the WinML backend only supports tensors, found: {:?}",
                    output_feature.Kind()
                ),
            };
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
fn to_inspectable(tensor: &Tensor) -> Result<IInspectable, Error> {
    let shape = IVectorView::<i64>::try_from(
        tensor
            .dimensions
            .iter()
            .map(|&x| x as i64)
            .collect::<Vec<i64>>(),
    )?;
    match tensor.ty {
        // f16 is not official supported by stable version of Rust. https://github.com/rust-lang/rust/issues/116909
        // Therefore we create TensorFloat16Bit from f32 array. https://microsoft.github.io/windows-docs-rs/doc/windows/AI/MachineLearning/struct.TensorFloat16Bit.html#method.CreateFromArray
        TensorType::Fp16 => unsafe {
            let data = std::slice::from_raw_parts(
                tensor.data.as_ptr().cast::<f32>(),
                tensor.data.len() / size_of::<f32>(),
            );
            check_alignment::<f32>(data);
            TensorFloat16Bit::CreateFromArray(&shape, data)?.cast::<IInspectable>()
        },
        TensorType::Fp32 => unsafe {
            let data = std::slice::from_raw_parts(
                tensor.data.as_ptr().cast::<f32>(),
                tensor.data.len() / size_of::<f32>(),
            );
            check_alignment::<f32>(data);
            TensorFloat::CreateFromArray(&shape, data)?.cast::<IInspectable>()
        },
        TensorType::I64 => unsafe {
            let data = std::slice::from_raw_parts(
                tensor.data.as_ptr().cast::<i64>(),
                tensor.data.len() / size_of::<i64>(),
            );
            check_alignment::<i64>(data);
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
            let data = view.into_iter().flat_map(f32::to_le_bytes).collect();
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
            let data = view.into_iter().flat_map(f32::to_le_bytes).collect();
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
            let data = view.into_iter().flat_map(i64::to_le_bytes).collect();
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

fn check_alignment<T>(data: &[T]) {
    let (prefix, _slice, suffix) = unsafe { data.align_to::<T>() };
    assert!(
        prefix.is_empty() && suffix.is_empty(),
        "Data is not aligned to {:?}'s alignment",
        std::any::type_name::<T>()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests for different data types. Convert from wasi-nn tensor to WinML tensor and back.
    #[test]
    fn fp16() {
        let data = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let buffer = data
            .iter()
            .map(|f| f.to_ne_bytes())
            .flatten()
            .collect::<Vec<u8>>();
        let buffer_copy = buffer.clone();
        let tensor = Tensor {
            ty: TensorType::Fp16,
            dimensions: vec![2, 3],
            data: buffer_copy,
        };
        let inspectable = to_inspectable(&tensor);
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
        assert_eq!(t.unwrap(), tensor);
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
        let inspectable = to_inspectable(&tensor);
        assert!(inspectable.is_ok());
        let winml_tensor = inspectable.as_ref().unwrap().cast::<TensorFloat>().unwrap();
        let view = winml_tensor.GetAsVectorView().unwrap();
        assert_eq!(view.into_iter().collect::<Vec<f32>>(), data);
        // Convert back.
        let t = to_tensor(inspectable.unwrap(), TensorKind::Float);
        assert!(t.as_ref().is_ok());
        assert_eq!(t.unwrap(), tensor);
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
        let inspectable = to_inspectable(&tensor);
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
        assert_eq!(t.unwrap(), tensor);
    }
}
