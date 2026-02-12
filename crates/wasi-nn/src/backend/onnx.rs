//! Implements a `wasi-nn` [`BackendInner`] using ONNX via the `ort` crate.

use super::{
    BackendError, BackendExecutionContext, BackendFromDir, BackendGraph, BackendInner, NamedTensor,
};
use crate::backend::{Id, read};
use crate::wit::types::{ExecutionTarget, GraphEncoding, Tensor, TensorType};
use crate::{ExecutionContext, Graph};
use ort::{
    execution_providers::{CPUExecutionProvider, ExecutionProviderDispatch},
    inputs,
    session::{Input, Output},
    session::{Session, SessionInputValue, builder::GraphOptimizationLevel},
    tensor::TensorElementType,
    value::{Tensor as OrtTensor, ValueType},
};

#[cfg(feature = "onnx-cuda")]
use ort::execution_providers::CUDAExecutionProvider;

use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct OnnxBackend();
unsafe impl Send for OnnxBackend {}
unsafe impl Sync for OnnxBackend {}

impl BackendInner for OnnxBackend {
    fn encoding(&self) -> GraphEncoding {
        GraphEncoding::Onnx
    }

    fn load(&mut self, builders: &[&[u8]], target: ExecutionTarget) -> Result<Graph, BackendError> {
        if builders.len() != 1 {
            return Err(BackendError::InvalidNumberOfBuilders(1, builders.len()));
        }

        // Configure execution providers based on target
        let execution_providers = configure_execution_providers(target)?;

        let session = Session::builder()?
            .with_execution_providers(execution_providers)?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .commit_from_memory(builders[0])?;

        let box_: Box<dyn BackendGraph> =
            Box::new(OnnxGraph(Arc::new(Mutex::new(session)), target));
        Ok(box_.into())
    }

    fn as_dir_loadable<'a>(&'a mut self) -> Option<&'a mut dyn BackendFromDir> {
        Some(self)
    }
}

/// Configure execution providers based on the target
fn configure_execution_providers(
    target: ExecutionTarget,
) -> Result<Vec<ExecutionProviderDispatch>, BackendError> {
    match target {
        ExecutionTarget::Cpu => {
            // Use CPU execution provider with default configuration
            tracing::debug!("Using CPU execution provider");
            Ok(vec![CPUExecutionProvider::default().build()])
        }
        ExecutionTarget::Gpu => {
            #[cfg(feature = "onnx-cuda")]
            {
                // Use CUDA execution provider for GPU acceleration
                tracing::debug!("Using Nvidia GPU CUDA execution provider");
                Ok(vec![CUDAExecutionProvider::default().build()])
            }
            #[cfg(not(feature = "onnx-cuda"))]
            {
                tracing::warn!("GPU CUDA execution provider is not enabled, falling back to CPU");
                Ok(vec![CPUExecutionProvider::default().build()])
            }
        }
        ExecutionTarget::Tpu => {
            tracing::warn!(
                "TPU execution target is not supported for ONNX backend yet, falling back to CPU"
            );
            Ok(vec![CPUExecutionProvider::default().build()])
        }
    }
}

impl BackendFromDir for OnnxBackend {
    fn load_from_dir(
        &mut self,
        path: &Path,
        target: ExecutionTarget,
    ) -> Result<Graph, BackendError> {
        let model = read(&path.join("model.onnx"))?;
        self.load(&[&model], target)
    }
}

struct OnnxGraph(Arc<Mutex<Session>>, #[allow(dead_code)] ExecutionTarget);
unsafe impl Send for OnnxGraph {}
unsafe impl Sync for OnnxGraph {}

impl BackendGraph for OnnxGraph {
    fn init_execution_context(&self) -> Result<ExecutionContext, BackendError> {
        let session = self.0.lock().unwrap();
        // We need to hold on to the names of the inputs in order for
        // `set_input` to work with both indexes and names. Having the
        // dimensions and type around is useful for validation but could be
        // retrieved from the session.
        let mut inputs = vec![];
        for input in &session.inputs {
            let shape = Shape::from_onnx_input(input)?;
            inputs.push(TensorSlot {
                shape,
                tensor: None,
            });
        }
        // We need to keep track of the output shapes since they are used for
        // creating the output tensor.
        let mut outputs = vec![];
        for output in &session.outputs {
            let shape = Shape::from_onnx_output(output)?;
            outputs.push(TensorSlot {
                shape,
                tensor: None,
            });
        }
        let box_: Box<dyn BackendExecutionContext> = Box::new(OnnxExecutionContext {
            session: self.0.clone(),
            inputs,
            outputs,
        });
        Ok(box_.into())
    }
}

struct OnnxExecutionContext {
    session: Arc<Mutex<Session>>,
    inputs: Vec<TensorSlot>,
    outputs: Vec<TensorSlot>,
}

unsafe impl Send for OnnxExecutionContext {}
unsafe impl Sync for OnnxExecutionContext {}

impl OnnxExecutionContext {
    /// Helper function for finding the internal index of a tensor by [`Id`].
    fn find(&self, id: Id, list: &[TensorSlot]) -> Result<usize, BackendError> {
        let index = match id {
            Id::Index(i) => {
                let i = i as usize;
                if i < list.len() {
                    i
                } else {
                    return Err(BackendError::BackendAccess(wasmtime::format_err!(
                        "incorrect tensor index: {i} >= {}",
                        list.len()
                    )));
                }
            }
            Id::Name(n) => list.iter().position(|s| s.shape.name == n).ok_or_else(|| {
                BackendError::BackendAccess(wasmtime::format_err!("unknown tensor name: {n}"))
            })?,
        };
        Ok(index)
    }
}

impl BackendExecutionContext for OnnxExecutionContext {
    fn set_input(&mut self, id: Id, tensor: &Tensor) -> Result<(), BackendError> {
        let index = self.find(id, &self.inputs)?;
        let input = &mut self.inputs[index];
        if let Err(e) = input.shape.matches(tensor) {
            return Err(e.into());
        }
        // Hold the tensor data on the context until `compute` is called.
        input.tensor.replace(tensor.clone());
        Ok(())
    }

    fn compute(
        &mut self,
        inputs: Option<Vec<NamedTensor>>,
    ) -> Result<Option<Vec<NamedTensor>>, BackendError> {
        fn dimensions_as_u32(shape: &ort::tensor::Shape) -> Result<Vec<u32>, BackendError> {
            (*shape)
                .iter()
                .map(|d| if *d == -1 { Ok(1) } else { convert_i64(d) })
                .collect()
        }

        match inputs {
            // WIT
            Some(inputs) => {
                for slot in &mut self.inputs {
                    slot.tensor = None;
                }
                for input in &inputs {
                    let index = self
                        .inputs
                        .iter()
                        .position(|slot| slot.shape.name == input.name);
                    let index = match index {
                        Some(idx) => idx,
                        None => {
                            // Try to convert name to index
                            if let Ok(idx) = input.name.parse::<usize>() {
                                if idx < self.inputs.len() {
                                    idx
                                } else {
                                    return Err(BackendError::BackendAccess(
                                        wasmtime::format_err!("Input index out of range: {idx}"),
                                    ));
                                }
                            } else {
                                return Err(BackendError::BackendAccess(wasmtime::format_err!(
                                    "Unknown input tensor name: {}",
                                    input.name
                                )));
                            }
                        }
                    };

                    let input_slot = &mut self.inputs[index];
                    if let Err(e) = input_slot.shape.matches(&input.tensor) {
                        return Err(e.into());
                    }
                    input_slot.tensor.replace(input.tensor.clone());
                }

                let mut session_inputs: Vec<SessionInputValue<'_>> = vec![];
                for i in &self.inputs {
                    session_inputs.extend(to_input_value(i)?);
                }
                let mut session = self.session.lock().unwrap();
                let session_outputs = session.run(session_inputs.as_slice())?;

                let mut output_tensors = Vec::new();
                for i in 0..self.outputs.len() {
                    // TODO: fix preexisting gap--this only handles f32 tensors.
                    let (shape, data): (&ort::tensor::Shape, &[f32]) =
                        session_outputs[i].try_extract_tensor()?;
                    let f32s = data.to_vec();
                    let output = &mut self.outputs[i];
                    let dimensions: Vec<u32> = dimensions_as_u32(shape)?;
                    let tensor = Tensor {
                        dimensions,
                        ty: output.shape.ty,
                        data: f32_vec_to_bytes(f32s),
                    };
                    output.tensor.replace(tensor.clone());
                    output_tensors.push(NamedTensor {
                        name: output.shape.name.clone(),
                        tensor,
                    });
                }
                Ok(Some(output_tensors))
            }

            // WITX
            None => {
                let mut session_inputs: Vec<SessionInputValue<'_>> = vec![];
                for i in &self.inputs {
                    session_inputs.extend(to_input_value(i)?);
                }
                let mut session = self.session.lock().unwrap();
                let session_outputs = session.run(session_inputs.as_slice())?;
                for i in 0..self.outputs.len() {
                    // TODO: fix preexisting gap--this only handles f32 tensors.
                    let (shape, data): (&ort::tensor::Shape, &[f32]) =
                        session_outputs[i].try_extract_tensor()?;
                    let f32s = data.to_vec();
                    let output = &mut self.outputs[i];
                    let dimensions: Vec<u32> = dimensions_as_u32(shape)?;
                    output.tensor.replace(Tensor {
                        dimensions,
                        ty: output.shape.ty,
                        data: f32_vec_to_bytes(f32s),
                    });
                }
                Ok(None)
            }
        }
    }

    fn get_output(&mut self, id: Id) -> Result<Tensor, BackendError> {
        let index = self.find(id, &self.outputs)?;
        let output = &self.outputs[index];
        if let Some(tensor) = &output.tensor {
            Ok(tensor.clone())
        } else {
            Err(BackendError::BackendAccess(wasmtime::format_err!(
                "missing output tensor: {}; has `compute` been called?",
                output.shape.name
            )))
        }
    }
}

impl From<ort::Error> for BackendError {
    fn from(e: ort::Error) -> Self {
        BackendError::BackendAccess(wasmtime::format_err!("{e}"))
    }
}

/// Holds a slot for ONNX session inputs and outputs.
///
/// TODO: it seems unfortunate that we have to "hold" some extra data per
/// session but in the input case, this is necessary for name-based indexing.
struct TensorSlot {
    shape: Shape,
    tensor: Option<Tensor>,
}

/// Describes a tensor in ONNX terms.
struct Shape {
    name: String,
    dimensions: Vec<i64>,
    ty: TensorType,
}

impl Shape {
    fn from_onnx_input(input: &Input) -> Result<Self, BackendError> {
        let name = input.name.clone();
        let (dimensions, ty) = convert_value_type(&input.input_type)?;
        Ok(Self {
            name,
            dimensions,
            ty,
        })
    }

    fn from_onnx_output(output: &Output) -> Result<Self, BackendError> {
        let name = output.name.clone();
        let (dimensions, ty) = convert_value_type(&output.output_type)?;
        Ok(Self {
            name,
            dimensions,
            ty,
        })
    }

    fn matches(&self, tensor: &Tensor) -> wasmtime::Result<()> {
        if self.dimensions.len() != tensor.dimensions.len() {
            return Err(wasmtime::format_err!(
                "input tensor cardinality does not match model: {:?} != {:?}",
                self.dimensions,
                tensor.dimensions
            ));
        } else {
            for (&shape_dim, &tensor_dim) in self.dimensions.iter().zip(tensor.dimensions.iter()) {
                let tensor_dim = tensor_dim as i64;
                if !is_dynamic_dimension(shape_dim) && shape_dim != tensor_dim {
                    return Err(wasmtime::format_err!(
                        "input tensor dimensions do not match model: {:?} != {:?}",
                        self.dimensions,
                        tensor.dimensions
                    ));
                }
            }
        }
        if self.ty != tensor.ty {
            return Err(wasmtime::format_err!(
                "input tensor type does not match model: {:?} != {:?}",
                self.ty,
                tensor.ty
            ));
        }
        Ok(())
    }
}

fn convert_value_type(vt: &ValueType) -> Result<(Vec<i64>, TensorType), BackendError> {
    match vt {
        ValueType::Tensor { ty, shape, .. } => {
            let dimensions = shape.to_vec();
            let ty = (*ty).try_into()?;
            Ok((dimensions, ty))
        }
        _ => Err(BackendError::BackendAccess(wasmtime::format_err!(
            "unsupported input type: {vt:?}"
        ))),
    }
}

fn convert_i64(i: &i64) -> Result<u32, BackendError> {
    u32::try_from(*i).map_err(|d| -> BackendError {
        wasmtime::format_err!("unable to convert dimension to u32: {d}").into()
    })
}

impl TryFrom<TensorElementType> for TensorType {
    type Error = BackendError;
    fn try_from(ty: TensorElementType) -> Result<Self, Self::Error> {
        match ty {
            TensorElementType::Float32 => Ok(TensorType::Fp32),
            TensorElementType::Float64 => Ok(TensorType::Fp64),
            TensorElementType::Uint8 => Ok(TensorType::U8),
            TensorElementType::Int32 => Ok(TensorType::I32),
            TensorElementType::Int64 => Ok(TensorType::I64),
            _ => Err(BackendError::BackendAccess(wasmtime::format_err!(
                "unsupported tensor type: {ty:?}"
            ))),
        }
    }
}

fn to_input_value(slot: &TensorSlot) -> Result<[SessionInputValue<'_>; 1], BackendError> {
    match &slot.tensor {
        Some(tensor) => match tensor.ty {
            TensorType::Fp32 => {
                let data = bytes_to_f32_vec(tensor.data.to_vec());
                let dimensions: Vec<i64> = tensor
                    .dimensions
                    .iter()
                    .map(|d| *d as i64) // TODO: fewer conversions
                    .collect();
                let ort_tensor = OrtTensor::<f32>::from_array((dimensions, data)).map_err(|e| {
                    BackendError::BackendAccess(wasmtime::format_err!(
                        "failed to create ONNX session input: {e}"
                    ))
                })?;
                Ok(inputs![ort_tensor])
            }
            _ => {
                unimplemented!("{:?} not supported by ONNX", tensor.ty);
            }
        },
        None => {
            return Err(BackendError::BackendAccess(wasmtime::format_err!(
                "missing input tensor: {}",
                slot.shape.name
            )));
        }
    }
}

pub fn f32_vec_to_bytes(data: Vec<f32>) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(data.len() * 4);
    for f in data {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

pub fn bytes_to_f32_vec(data: Vec<u8>) -> Vec<f32> {
    assert_eq!(data.len() % 4, 0);
    data.chunks(4)
        .map(|c| {
            let arr: [u8; 4] = c.try_into().unwrap();
            f32::from_le_bytes(arr)
        })
        .collect()
}

/// Returns whether the dimension is dynamic.
///
/// ONNX uses [dimensional variables] (i.e., name strings) to indicate that the
/// value of a tensor dimension is user-defined, not fixed by the model. This is
/// useful for batching up several inference requests, e.g. When `ort` returns a
/// dimension of this kind, though, it uses `-1` to indicate that the dimension
/// is dynamic.
///
/// [dimensional variables]:
///     https://onnx.ai/onnx/repo-docs/IR.html#static-tensor-shapes
fn is_dynamic_dimension(d: i64) -> bool {
    d == -1
}
