//! Implements a `wasi-nn` [`BackendInner`] using PyTorch.
//!
use super::{
    BackendError, BackendExecutionContext, BackendFromDir, BackendGraph, BackendInner, Id,
    NamedTensor,
};
use crate::wit::types::{ExecutionTarget, GraphEncoding, Tensor, TensorType};
use crate::{ExecutionContext, Graph};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tch::{CModule, Device, Kind, TchError, Tensor as TchTensor};

#[derive(Default)]
pub struct PytorchBackend();
unsafe impl Send for PytorchBackend {}
unsafe impl Sync for PytorchBackend {}

impl BackendInner for PytorchBackend {
    fn encoding(&self) -> GraphEncoding {
        GraphEncoding::Pytorch
    }

    fn load(&mut self, builders: &[&[u8]], target: ExecutionTarget) -> Result<Graph, BackendError> {
        if builders.len() != 1 {
            return Err(BackendError::InvalidNumberOfBuilders(1, builders.len()).into());
        }
        // Load the torchscript saved module.
        let mut saved_module = builders[0];

        // Load the saved model on the device.
        let mut compiled_module = CModule::load_data_on_device(
            &mut saved_module,
            map_execution_target_to_string(target),
        )?;

        // Set the model to be used for inference (eval), default mode is training.
        compiled_module.f_set_eval()?;

        let graph = PytorchGraph {
            module: Arc::new(Mutex::new(compiled_module)),
            target,
        };
        let box_: Box<dyn BackendGraph> = Box::new(graph);
        Ok(box_.into())
    }

    fn as_dir_loadable<'a>(&'a mut self) -> Option<&'a mut dyn BackendFromDir> {
        Some(self)
    }
}

impl BackendFromDir for PytorchBackend {
    fn load_from_dir(
        &mut self,
        path: &Path,
        target: ExecutionTarget,
    ) -> Result<Graph, BackendError> {
        // Load the model from the file path.
        let compiled_module = CModule::load_on_device(
            path.join("model.pt"),
            map_execution_target_to_string(target),
        )?;
        let graph = PytorchGraph {
            module: Arc::new(Mutex::new(compiled_module)),
            target,
        };
        let box_: Box<dyn BackendGraph> = Box::new(graph);
        Ok(box_.into())
    }
}

struct PytorchGraph {
    module: Arc<Mutex<tch::CModule>>,
    target: ExecutionTarget,
}

unsafe impl Send for PytorchGraph {}
unsafe impl Sync for PytorchGraph {}

impl BackendGraph for PytorchGraph {
    fn init_execution_context(&self) -> Result<ExecutionContext, BackendError> {
        let box_: Box<dyn BackendExecutionContext> = Box::new(PytorchExecutionContext {
            module: self.module.clone(),
            inputs: Vec::new(),
            output: TchTensor::new(),
            id_type: None,
            target: self.target,
        });

        Ok(box_.into())
    }
}

unsafe impl Sync for PytorchExecutionContext {}
struct PytorchExecutionContext {
    module: Arc<Mutex<tch::CModule>>,
    inputs: Vec<Option<tch::Tensor>>,
    output: tch::Tensor,
    id_type: Option<Id>,
    target: ExecutionTarget,
}

/// `set_input` supports multiple positional parameters with `Id::Index`, and a single named parameter with `Id::Name`.
/// `set_input` may be removed in the future, with `compute` method taking a list of named parameters.
/// See [PR #77](https://github.com/WebAssembly/wasi-nn/pull/77), at which point multiple named parameters for `Tensor` inputs is planned to be supported in pytorch backend.
impl BackendExecutionContext for PytorchExecutionContext {
    fn set_input(&mut self, id: Id, input_tensor: &Tensor) -> Result<(), BackendError> {
        let tensor = to_tch_tensor(input_tensor, self.target)?;
        match id {
            Id::Index(i) => {
                // Check if id_type is already set and if it matches the current id type
                if let Some(Id::Name(_)) = self.id_type {
                    return Err(BackendError::BackendAccess(wasmtime::format_err!(
                        "Cannot mix u32 and str indexes"
                    )));
                }
                // Set id_type if not already set
                if self.id_type.is_none() {
                    self.id_type = Some(Id::Index(0)); // Provide a u32 value for Index
                }
                let i = i as usize;
                if i >= self.inputs.len() {
                    self.inputs.resize_with(i + 1, || None);
                }
                self.inputs[i] = Some(tensor);
                Ok(())
            }
            Id::Name(_) => {
                // Check if id_type is already set and if it matches the current id type
                if let Some(Id::Index(_)) = self.id_type {
                    return Err(BackendError::BackendAccess(wasmtime::format_err!(
                        "Cannot mix u32 and str indexes"
                    )));
                }
                // Set id_type if not already set
                if self.id_type.is_none() {
                    self.id_type = Some(Id::Name(String::new())); // Provide a str value for Name
                }
                if self.inputs.get(0).is_some() {
                    return Err(BackendError::BackendAccess(wasmtime::format_err!(
                        "The pytorch backend does not support multiple named inputs"
                    )));
                } else {
                    self.inputs.push(Some(tensor));
                }
                Ok(())
            }
        }
    }

    fn compute(
        &mut self,
        inputs: Option<Vec<NamedTensor>>,
    ) -> Result<Option<Vec<NamedTensor>>, BackendError> {
        match inputs {
            // WIT-style compute with named tensors
            Some(inputs) => {
                self.inputs.clear();
                self.id_type = None;
                for input in &inputs {
                    let tensor = to_tch_tensor(&input.tensor, self.target)?;
                    self.inputs.push(Some(tensor));

                    // Set ID type to Name since we're using named tensors
                    if self.id_type.is_none() {
                        self.id_type = Some(Id::Name(String::new()));
                    }
                }
                // Run the forward pass
                let inputs: Vec<tch::Tensor> = self
                    .inputs
                    .iter()
                    .enumerate()
                    .map(|(index, opt)| {
                        opt.as_ref()
                            .expect(&format!("Input tensor at index {} not set up", index))
                            .shallow_clone()
                    })
                    .collect();
                self.output = self.module.lock().unwrap().forward_ts(&inputs)?;
                let numel = self.output.numel();
                let dimensions = self.output.size();
                let ty = self.output.kind().try_into()?;
                let mut data = vec![0u8; kind_to_size(self.output.kind())? * numel];
                self.output.copy_data_u8(&mut data, numel);
                let output_tensor = Tensor {
                    dimensions: dimensions.iter().map(|&dim| dim as u32).collect(),
                    ty,
                    data,
                };
                let output = NamedTensor {
                    name: "output".to_string(),
                    tensor: output_tensor,
                };
                Ok(Some(vec![output]))
            }

            // WITX-style compute with previously set inputs
            None => {
                if self.inputs.is_empty() {
                    return Err(BackendError::BackendAccess(wasmtime::format_err!(
                        "No inputs provided for inference"
                    )));
                }
                let inputs: Vec<tch::Tensor> = self
                    .inputs
                    .iter()
                    .enumerate()
                    .map(|(index, opt)| {
                        opt.as_ref()
                            .expect(&format!("Input tensor at index {} not set up", index))
                            .shallow_clone()
                    })
                    .collect();
                // Perform forward pass
                self.output = self.module.lock().unwrap().forward_ts(&inputs)?;
                Ok(None)
            }
        }
    }

    fn get_output(&mut self, _index: Id) -> Result<Tensor, BackendError> {
        // Output index is not used. The forward_ts method to a model returns a single output tensor.
        let numel = self.output.numel();
        let dimensions = self.output.size();
        let ty = self.output.kind().try_into()?;
        let mut data = vec![0u8; kind_to_size(self.output.kind())? * numel];
        self.output.copy_data_u8(&mut data, numel);
        Ok(Tensor {
            dimensions: dimensions.iter().map(|&dim| dim as u32).collect(),
            ty,
            data,
        })
    }
}

/// Converts a wasi-nn [`Tensor`] into a PyTorch tensor on `target`.
///
/// `TchTensor::from_data_size` copies as many bytes as the dimensions and
/// element type call for without looking at the length of the data, so check
/// that the guest-provided data is exactly that size first.
fn to_tch_tensor(tensor: &Tensor, target: ExecutionTarget) -> Result<TchTensor, BackendError> {
    let kind: Kind = tensor.ty.try_into()?;
    let expected = tensor
        .dimensions
        .iter()
        .try_fold(kind.elt_size_in_bytes(), |count, &dim| {
            count.checked_mul(usize::try_from(dim).ok()?)
        })
        .ok_or_else(|| {
            BackendError::BackendAccess(wasmtime::format_err!(
                "tensor dimensions are too large: {:?}",
                tensor.dimensions
            ))
        })?;
    if tensor.data.len() != expected {
        return Err(BackendError::BackendAccess(wasmtime::format_err!(
            "input tensor has {} bytes of data but dimensions {:?} of type {:?} need {expected} bytes",
            tensor.data.len(),
            tensor.dimensions,
            tensor.ty
        )));
    }
    let dimensions = tensor
        .dimensions
        .iter()
        .map(|&dim| dim as i64)
        .collect::<Vec<_>>();
    Ok(TchTensor::from_data_size(&tensor.data, &dimensions, kind)
        .to_device(map_execution_target_to_string(target)))
}

fn map_execution_target_to_string(target: ExecutionTarget) -> Device {
    match target {
        ExecutionTarget::Cpu => Device::Cpu,
        ExecutionTarget::Gpu => Device::Cuda(0),
        ExecutionTarget::Tpu => {
            unimplemented!("the pytorch backend does not yet support TPU execution targets")
        }
    }
}

fn kind_to_size(kind: Kind) -> Result<usize, BackendError> {
    match kind {
        Kind::Float | Kind::Half => Ok(std::mem::size_of::<f32>()), // f16 is unstable https://github.com/rust-lang/rust/issues/116909
        Kind::Double => Ok(std::mem::size_of::<f64>()),
        Kind::Int => Ok(std::mem::size_of::<i32>()),
        Kind::Uint8 => Ok(std::mem::size_of::<u8>()),
        Kind::Int64 => Ok(std::mem::size_of::<i64>()),
        _ => Err(BackendError::UnsupportedTensorType(format!("{:?}", kind))),
    }
}

/// Returns the PyTorch [`Kind`] from wasi-nn's [`TensorType`].
impl TryFrom<TensorType> for Kind {
    type Error = BackendError;

    fn try_from(tensor_type: TensorType) -> Result<Self, Self::Error> {
        match tensor_type {
            TensorType::Fp16 => Ok(Kind::Half),
            TensorType::Fp32 => Ok(Kind::Float),
            TensorType::Fp64 => Ok(Kind::Double),
            TensorType::U8 => Ok(Kind::Uint8),
            TensorType::I32 => Ok(Kind::Int),
            TensorType::I64 => Ok(Kind::Int64),
            _ => Err(BackendError::UnsupportedTensorType(format!(
                "{:?}",
                tensor_type
            ))),
        }
    }
}

/// Returns wasi-nn [`TensorType`] from PyTorch's [`Kind`].
impl TryFrom<Kind> for TensorType {
    type Error = BackendError;

    fn try_from(kind: Kind) -> Result<Self, Self::Error> {
        match kind {
            Kind::Half => Ok(TensorType::Fp16),
            Kind::Float => Ok(TensorType::Fp32),
            Kind::Double => Ok(TensorType::Fp64),
            Kind::Uint8 => Ok(TensorType::U8),
            Kind::Int => Ok(TensorType::I32),
            Kind::Int64 => Ok(TensorType::I64),
            _ => Err(BackendError::UnsupportedTensorType(format!("{:?}", kind))),
        }
    }
}

impl From<TchError> for BackendError {
    fn from(e: TchError) -> Self {
        BackendError::BackendAccess(wasmtime::Error::new(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_tch_tensor_checks_data_length() {
        let exact = Tensor::new(vec![2, 3], TensorType::Fp32, vec![0; 24]);
        assert!(to_tch_tensor(&exact, ExecutionTarget::Cpu).is_ok());

        let short = Tensor::new(vec![4096], TensorType::Fp32, vec![0; 4]);
        assert!(to_tch_tensor(&short, ExecutionTarget::Cpu).is_err());

        let long = Tensor::new(vec![2], TensorType::Fp16, vec![0; 8]);
        assert!(to_tch_tensor(&long, ExecutionTarget::Cpu).is_err());

        let huge = Tensor::new(vec![u32::MAX; 4], TensorType::I64, vec![]);
        assert!(to_tch_tensor(&huge, ExecutionTarget::Cpu).is_err());
    }
}
