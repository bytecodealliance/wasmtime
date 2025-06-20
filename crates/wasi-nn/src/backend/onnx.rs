//! Implements a `wasi-nn` [`BackendInner`] using ONNX via the `ort` crate.

use super::{BackendError, BackendExecutionContext, BackendFromDir, BackendGraph, BackendInner};
use crate::backend::{Id, read};
use crate::wit::types::{ExecutionTarget, GraphEncoding, Tensor, TensorType};
use crate::{ExecutionContext, Graph};
use anyhow::Context;
use ort::{GraphOptimizationLevel, Session, inputs};
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
            return Err(BackendError::InvalidNumberOfBuilders(1, builders.len()).into());
        }

        let session = Session::builder()?
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
                    return Err(BackendError::BackendAccess(anyhow::anyhow!(
                        "incorrect tensor index: {i} >= {}",
                        list.len()
                    )));
                }
            }
            Id::Name(n) => list.iter().position(|s| s.shape.name == n).ok_or_else(|| {
                BackendError::BackendAccess(anyhow::anyhow!("unknown tensor name: {n}"))
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

    fn compute(&mut self) -> Result<(), BackendError> {
        let mut session_inputs: Vec<ort::SessionInputValue<'_>> = vec![];
        for i in &self.inputs {
            session_inputs.extend(to_input_value(i)?);
        }
        let session = self.session.lock().unwrap();
        let session_outputs = session.run(session_inputs.as_slice())?;
        for i in 0..self.outputs.len() {
            // TODO: fix preexisting gap--this only handles f32 tensors.
            let raw: (Vec<i64>, &[f32]) = session_outputs[i].try_extract_raw_tensor()?;
            let f32s = raw.1.to_vec();
            let output = &mut self.outputs[i];
            output.tensor.replace(Tensor {
                dimensions: output.shape.dimensions_as_u32()?,
                ty: output.shape.ty,
                data: f32_vec_to_bytes(f32s),
            });
        }
        Ok(())
    }

    fn get_output(&mut self, id: Id) -> Result<Tensor, BackendError> {
        let index = self.find(id, &self.outputs)?;
        let output = &self.outputs[index];
        if let Some(tensor) = &output.tensor {
            Ok(tensor.clone())
        } else {
            Err(BackendError::BackendAccess(anyhow::anyhow!(
                "missing output tensor: {}; has `compute` been called?",
                output.shape.name
            )))
        }
    }
}

impl From<ort::Error> for BackendError {
    fn from(e: ort::Error) -> Self {
        BackendError::BackendAccess(e.into())
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
    fn from_onnx_input(input: &ort::Input) -> Result<Self, BackendError> {
        let name = input.name.clone();
        let (dimensions, ty) = convert_value_type(&input.input_type)?;
        Ok(Self {
            name,
            dimensions,
            ty,
        })
    }

    fn from_onnx_output(output: &ort::Output) -> Result<Self, BackendError> {
        let name = output.name.clone();
        let (dimensions, ty) = convert_value_type(&output.output_type)?;
        Ok(Self {
            name,
            dimensions,
            ty,
        })
    }

    fn dimensions_as_u32(&self) -> Result<Vec<u32>, BackendError> {
        self.dimensions
            .iter()
            .map(|d| if *d == -1 { Ok(1) } else { convert_i64(d) })
            .collect()
    }

    fn matches(&self, tensor: &Tensor) -> anyhow::Result<()> {
        if self.dimensions.len() != tensor.dimensions.len() {
            return Err(anyhow::anyhow!(
                "input tensor cardinality does not match model: {:?} != {:?}",
                self.dimensions,
                tensor.dimensions
            ));
        } else {
            for (&shape_dim, &tensor_dim) in self.dimensions.iter().zip(tensor.dimensions.iter()) {
                let tensor_dim = tensor_dim as i64;
                if !is_dynamic_dimension(shape_dim) && shape_dim != tensor_dim {
                    return Err(anyhow::anyhow!(
                        "input tensor dimensions do not match model: {:?} != {:?}",
                        self.dimensions,
                        tensor.dimensions
                    ));
                }
            }
        }
        if self.ty != tensor.ty {
            return Err(anyhow::anyhow!(
                "input tensor type does not match model: {:?} != {:?}",
                self.ty,
                tensor.ty
            ));
        }
        Ok(())
    }
}

fn convert_value_type(vt: &ort::ValueType) -> Result<(Vec<i64>, TensorType), BackendError> {
    match vt {
        ort::ValueType::Tensor { ty, dimensions } => {
            let dims = dimensions.clone();
            let ty = (*ty).try_into()?;
            Ok((dims, ty))
        }
        _ => Err(BackendError::BackendAccess(anyhow::anyhow!(
            "unsupported input type: {vt:?}"
        ))),
    }
}

fn convert_i64(i: &i64) -> Result<u32, BackendError> {
    u32::try_from(*i).map_err(|d| -> BackendError {
        anyhow::anyhow!("unable to convert dimension to u32: {d}").into()
    })
}

impl TryFrom<ort::TensorElementType> for TensorType {
    type Error = BackendError;
    fn try_from(ty: ort::TensorElementType) -> Result<Self, Self::Error> {
        match ty {
            ort::TensorElementType::Float32 => Ok(TensorType::Fp32),
            ort::TensorElementType::Float64 => Ok(TensorType::Fp64),
            ort::TensorElementType::Uint8 => Ok(TensorType::U8),
            ort::TensorElementType::Int32 => Ok(TensorType::I32),
            ort::TensorElementType::Int64 => Ok(TensorType::I64),
            _ => Err(BackendError::BackendAccess(anyhow::anyhow!(
                "unsupported tensor type: {ty:?}"
            ))),
        }
    }
}

fn to_input_value(slot: &TensorSlot) -> Result<[ort::SessionInputValue<'_>; 1], BackendError> {
    match &slot.tensor {
        Some(tensor) => match tensor.ty {
            TensorType::Fp32 => {
                let data = bytes_to_f32_vec(tensor.data.to_vec());
                let dimensions = tensor
                    .dimensions
                    .iter()
                    .map(|d| *d as i64) // TODO: fewer conversions
                    .collect::<Vec<i64>>();
                Ok(inputs![(dimensions, Arc::new(data.into_boxed_slice()))]
                    .context("failed to create ONNX session input")?)
            }
            _ => {
                unimplemented!("{:?} not supported by ONNX", tensor.ty);
            }
        },
        None => {
            return Err(BackendError::BackendAccess(anyhow::anyhow!(
                "missing input tensor: {}",
                slot.shape.name
            )));
        }
    }
}

pub fn f32_vec_to_bytes(data: Vec<f32>) -> Vec<u8> {
    let chunks: Vec<[u8; 4]> = data.into_iter().map(|f| f.to_le_bytes()).collect();
    let result: Vec<u8> = chunks.iter().flatten().copied().collect();
    result
}

pub fn bytes_to_f32_vec(data: Vec<u8>) -> Vec<f32> {
    let chunks: Vec<&[u8]> = data.chunks(4).collect();
    let v: Vec<f32> = chunks
        .into_iter()
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect();

    v.into_iter().collect()
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
