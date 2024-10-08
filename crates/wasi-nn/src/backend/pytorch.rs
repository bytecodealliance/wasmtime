//! Implements a `wasi-nn` [`BackendInner`] using PyTorch.
//!
use super::{
    BackendError, BackendExecutionContext, BackendFromDir, BackendGraph, BackendInner, Id,
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
        };
        let box_: Box<dyn BackendGraph> = Box::new(graph);
        Ok(box_.into())
    }
}

struct PytorchGraph {
    module: Arc<Mutex<tch::CModule>>,
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
}

impl BackendExecutionContext for PytorchExecutionContext {
    fn set_input(&mut self, id: Id, input_tensor: &Tensor) -> Result<(), BackendError> {
        let kind = input_tensor.ty.try_into()?;
        let dimensions = input_tensor
            .dimensions
            .iter()
            .map(|&dim| dim as i64)
            .collect::<Vec<_>>();
        let tensor = TchTensor::from_data_size(&input_tensor.data, &dimensions, kind);
        match id {
            Id::Index(i) => {
                // Check if id_type is already set and if it matches the current id type
                if let Some(Id::Name(_)) = self.id_type {
                    return Err(BackendError::BackendAccess(anyhow::anyhow!(
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
                    return Err(BackendError::BackendAccess(anyhow::anyhow!(
                        "Cannot mix u32 and str indexes"
                    )));
                }
                // Set id_type if not already set
                if self.id_type.is_none() {
                    self.id_type = Some(Id::Name(String::new())); // Provide a str value for Name
                }
                if self.inputs.get(0).is_some() {
                    return Err(BackendError::BackendAccess(anyhow::anyhow!(
                        "The pytorch backend does not support multiple named inputs"
                    )));
                } else {
                    self.inputs.push(Some(tensor));
                }
                Ok(())
            }
        }
    }

    fn compute(&mut self) -> Result<(), BackendError> {
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
        // Use forward_ts method on the compiled module/model after locking the mutex, and pass the input tensor to it
        self.output = self.module.lock().unwrap().forward_ts(&inputs).unwrap();
        Ok(())
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

fn map_execution_target_to_string(target: ExecutionTarget) -> Device {
    match target {
        ExecutionTarget::Cpu => Device::Cpu,
        ExecutionTarget::Gpu => {
            unimplemented!("the pytorch backend does not yet support GPU execution targets")
        }
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
        BackendError::BackendAccess(anyhow::Error::new(e))
    }
}
