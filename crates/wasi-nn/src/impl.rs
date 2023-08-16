//! Implements the wasi-nn API.

use crate::ctx::{LoadedModel, RegisteredModel, WasiNnResult as Result};
use crate::witx::types::{
    ExecutionTarget, Graph, GraphBuilderArray, GraphEncoding, GraphExecutionContext,
    Tensor,
};
use crate::witx::wasi_ephemeral_nn::WasiEphemeralNn;
use crate::WasiNnCtx;
use thiserror::Error;
use wiggle::GuestPtr;

const MAX_GUEST_MODEL_REGISTRATION_SIZE: usize = 20 * 1024 * 1024; //20M

#[derive(Debug, Error)]
pub enum UsageError {
    #[error("Invalid context; has the load function been called?")]
    InvalidContext,
    #[error("Only OpenVINO's IR is currently supported, passed encoding: {0:?}")]
    InvalidEncoding(GraphEncoding),
    #[error("OpenVINO expects only two buffers (i.e. [ir, weights]), passed: {0}")]
    InvalidNumberOfBuilders(u32),
    #[error("Invalid graph handle; has it been loaded?")]
    InvalidGraphHandle,
    #[error("Invalid execution context handle; has it been initialized?")]
    InvalidExecutionContextHandle,
    #[error("Not enough memory to copy tensor data of size: {0}")]
    NotEnoughMemory(u32),
    #[error("Model size {0} exceeds allowed quota of {1}")]
    ModelTooLarge(usize, usize),
}

impl WasiNnCtx {
    fn build_graph(
        &mut self,
        model_bytes: &Vec<Vec<u8>>,
        encoding: GraphEncoding,
        target: ExecutionTarget,
    ) -> Result<Graph> {
        let encoding_id: u8 = encoding.into();
        let graph = if let Some(backend) = self.backends.get_mut(&encoding_id) {
            backend.load_from_bytes(model_bytes, target)?
        } else {
            return Err(UsageError::InvalidEncoding(encoding).into());
        };

        let graph_id = self.graphs.insert(graph);
        Ok(graph_id)
    }
}

impl<'a> WasiEphemeralNn for WasiNnCtx {
    fn load<'b>(
        &mut self,
        builders: &GraphBuilderArray<'_>,
        encoding: GraphEncoding,
        target: ExecutionTarget
    ) -> Result<Graph> {
        let encoding_id: u8 = encoding.into();
        let graph = if let Some(backend) = self.backends.get_mut(&encoding_id) {
            backend.load(builders, target)?
        } else {
            return Err(UsageError::InvalidEncoding(encoding).into());
        };
        let graph_id = self.graphs.insert(graph);
        Ok(graph_id)
    }

    fn load_by_name<'b>(&mut self, model_name: &GuestPtr<'_,str>) -> Result<Graph> {
        let model_name = model_name.as_str().unwrap().unwrap().to_string();
        let maybe_loaded_model = self.loaded_models.get(&model_name);

        match maybe_loaded_model {
            Some(model) => Ok(model.graph),
            None => {
                let registered_model = self.model_registry.get(&model_name).unwrap();
                let model_bytes = &registered_model.model_bytes;
                let encoding: GraphEncoding = registered_model.encoding;
                let target: ExecutionTarget = registered_model.target;

                let encoding_id: u8 = encoding.into();
                let graph = if let Some(backend) = self.backends.get_mut(&encoding_id) {
                    backend.load_from_bytes(model_bytes, target)?
                } else {
                    return Err(UsageError::InvalidEncoding(encoding).into());
                };
                let graph_id = self.graphs.insert(graph);

                Ok(graph_id)
            }
        }
    }

    fn register_named_model(
        &mut self,
        model_name: &GuestPtr<'_, str>,
        model_bytes: &GraphBuilderArray<'_>,
        encoding: GraphEncoding,
        target: ExecutionTarget
    ) -> Result<()> {
        let length: usize = model_bytes.len().try_into().unwrap();
        if length > MAX_GUEST_MODEL_REGISTRATION_SIZE {
            return Err(
                UsageError::ModelTooLarge(length, MAX_GUEST_MODEL_REGISTRATION_SIZE).into(),
            );
        }

        let mut model_bytes_vec: Vec<Vec<u8>> = Vec::with_capacity(length.try_into().unwrap());
        let mut model_bytes = model_bytes.as_ptr();
        for _ in 0..length {
            let v = model_bytes
                .read()?
                .as_slice()?
                .expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)")
                .to_vec();
            model_bytes_vec.push(v);
            model_bytes = model_bytes.add(1)?;
        }
        let model_name_key = model_name.as_str().unwrap().unwrap().to_string();
        match target {
            ExecutionTarget::Cpu => {
                let graph = self.build_graph(&model_bytes_vec, encoding, target)?;
                self.loaded_models
                    .insert(model_name_key, LoadedModel { graph });
            }
            _ => {
                self.model_registry.insert(
                    model_name_key,
                    RegisteredModel {
                        model_bytes: model_bytes_vec,
                        encoding,
                        target
                    },
                );
            }
        };
        Ok(())
    }

    fn get_model_list<'b>(&mut self,
                          buffer: &GuestPtr<'b, u8>,
                          model_list: &GuestPtr<'b, GuestPtr<'b, u8>>,
                          length: u32) -> Result<()> {
        let mut model_names: Vec<String> = self.model_registry.iter().map(|e| e.key().to_string()).collect();
        self.loaded_models.iter().for_each(|e| model_names.push(e.key().to_string()));

        println!("Model names: {:?}", model_names);
        let model_names_array = StringArray { elems: model_names };
        model_names_array.write_to_guest(buffer, model_list);
        Ok(())
    }

    fn get_model_list_sizes(&mut self) -> Result<(u32, u32)> {
        let mut model_names: Vec<String> = self.model_registry.iter().map(|e| e.key().to_string()).collect();
        self.loaded_models.iter().for_each(|e| model_names.push(e.key().to_string()));
        let lengths: Vec<u32> = model_names.iter().map(|e| e.len() as u32).collect();
        let string_count = lengths.len() as u32;
        let buffer_size = lengths.iter().sum::<u32>() as u32 + string_count;
        Ok((string_count, buffer_size))
    }

    fn init_execution_context(&mut self, graph_id: Graph) -> Result<GraphExecutionContext> {
        let exec_context = if let Some(graph) = self.graphs.get_mut(graph_id) {
            graph.init_execution_context()?
        } else {
            return Err(UsageError::InvalidGraphHandle.into());
        };

        let exec_context_id = self.executions.insert(exec_context);
        Ok(exec_context_id)
    }

    fn set_input<'b>(
        &mut self,
        exec_context_id: GraphExecutionContext,
        index: u32,
        tensor: &Tensor<'b>,
    ) -> Result<()> {
        if let Some(exec_context) = self.executions.get_mut(exec_context_id) {
            Ok(exec_context.set_input(index, tensor)?)
        } else {
            Err(UsageError::InvalidGraphHandle.into())
        }
    }

    fn compute(&mut self, exec_context_id: GraphExecutionContext) -> Result<()> {
        if let Some(exec_context) = self.executions.get_mut(exec_context_id) {
            Ok(exec_context.compute()?)
        } else {
            Err(UsageError::InvalidExecutionContextHandle.into())
        }
    }

    fn get_output<'b>(
        &mut self,
        exec_context_id: GraphExecutionContext,
        index: u32,
        out_buffer: &GuestPtr<'_, u8>,
        out_buffer_max_size: u32,
    ) -> Result<u32> {
        if let Some(exec_context) = self.executions.get_mut(exec_context_id) {
            let mut destination = out_buffer
                .as_array(out_buffer_max_size)
                .as_slice_mut()?
                .expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)");
            Ok(exec_context.get_output(index, &mut destination)?)
        } else {
            Err(UsageError::InvalidGraphHandle.into())
        }
    }
}

pub struct StringArray {
    elems: Vec<String>,
}

impl StringArray {
    pub fn new() -> Self {
        StringArray { elems: Vec::new() }
    }

    pub fn number_elements(&self) -> u32 {
        self.elems.len() as u32
    }

    pub fn cumulative_size(&self) -> u32 {
        self.elems
            .iter()
            .map(|e| e.as_bytes().len() + 1)
            .sum::<usize>() as u32
    }

    pub fn write_to_guest<'a>(
        &self,
        buffer: &GuestPtr<'a, u8>,
        element_heads: &GuestPtr<'a, GuestPtr<'a, u8>>,
    ) -> Result<()> {
        println!("Model names to guest: {:?}", self.elems);
        let element_heads = element_heads.as_array(self.number_elements());
        let buffer = buffer.as_array(self.cumulative_size());
        let mut cursor = 0;
        for (elem, head) in self.elems.iter().zip(element_heads.iter()) {
            let bytes = elem.as_bytes();
            let len = bytes.len() as u32;
            {
                let elem_buffer = buffer
                    .get_range(cursor..(cursor + len))
                    .ok_or(UsageError::InvalidContext)?; // Elements don't fit in buffer provided
                elem_buffer.copy_from_slice(bytes)?;
            }
            buffer
                .get(cursor + len)
                .ok_or(UsageError::InvalidContext)?
                .write(0)?; // 0 terminate
            head?.write(buffer.get(cursor).expect("already validated"))?;
            cursor += len + 1;
        }
        Ok(())
    }
}