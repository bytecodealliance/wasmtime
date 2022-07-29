//! Implements the wasi-nn API.
use crate::api::{Backend, BackendError, BackendExecutionContext, BackendGraph};
use crate::witx::types::{ExecutionTarget, GraphBuilderArray, Tensor, TensorType};
use std::collections::HashMap;
use std::ops::Deref;
use std::path::Path;
use std::str;
use std::sync::Arc;
use tensorflow::{
    FetchToken, Graph, Operation, SavedModelBundle, SessionOptions, SessionRunArgs, Status,
};
use tensorflow::{SignatureDef, Tensor as TFTensor};
use tensorflow::{TensorInfo, DEFAULT_SERVING_SIGNATURE_DEF_KEY};

#[derive(Default)]
pub(crate) struct TensorflowBackend();

impl Backend for TensorflowBackend {
    fn name(&self) -> &str {
        "tensorflow"
    }

    fn load(
        &mut self,
        builders: &GraphBuilderArray<'_>,
        _target: ExecutionTarget,
        map_dir: &Option<Vec<(String, String)>>,
    ) -> Result<Box<dyn BackendGraph>, BackendError> {
        if let Some(dir) = map_dir {
            if builders.len() != 2 {
                return Err(BackendError::InvalidNumberOfBuilders(2, builders.len()).into());
            }
            // Initialize the Tensorflow backend
            let _retval = tensorflow::library::load().or_else(|e| {
                println!("Error loading the TensorFlow backend: \n {}", e);
                Err(e)
            });

            let mut graph = Graph::new();
            let builders = builders.as_ptr();
            let guest_map = builders.read()?.as_slice()?;
            let guest_map_str = str::from_utf8(&guest_map).unwrap();
            let exp_dir = builders.add(1)?.read()?.as_slice()?;
            let exp_str = str::from_utf8(&exp_dir).unwrap();

            // Don't allow navigation outside of the sandbox
            if !exp_str.contains("..") {
                for i in 0..dir.len() {
                    if dir[i].0 == guest_map_str {
                        //Append the stored mapdir path with the user path.
                        let full_path = std::fs::canonicalize(
                            Path::new(&dir[i].1.clone()).join(Path::new(exp_str)),
                        );

                        //Check that path actually exists
                        let full_path = match full_path {
                            Ok(fp) => fp,
                            Err(_e) => return Err(BackendError::MissingMapDir()),
                        };

                        let bundle = SavedModelBundle::load(
                            &SessionOptions::new(),
                            &["serve"],
                            &mut graph,
                            full_path,
                        )?;
                        return Ok(Box::new(TensorflowGraph(Arc::new(graph), Arc::new(bundle))));
                    }
                }
            }
        }

        Err(BackendError::MissingMapDir())
    }
}

struct TensorflowGraph(Arc<Graph>, Arc<SavedModelBundle>);

impl<'a> BackendGraph for TensorflowGraph {
    fn init_execution_context(&mut self) -> Result<Box<dyn BackendExecutionContext>, BackendError> {
        let signature = self
            .1
            .meta_graph_def()
            .get_signature(DEFAULT_SERVING_SIGNATURE_DEF_KEY)?;

        let mut outputs: Vec<String> = vec![];

        // Get the index of each output key
        for (key, _value) in signature.outputs() {
            outputs.push(key.clone());
        }

        // Get the indexes of the inputs in the signature
        let mut inputs: Vec<String> = vec![];
        for key in signature.inputs().keys() {
            inputs.push(key.clone());
        }

        // Currently we only support using one output, index == 0
        let info = signature.get_output(outputs[0].as_str())?.to_owned();
        Ok(Box::new(TensorflowExecutionContext {
            graph: self.0.clone(),
            bundle: self.1.clone(),
            tensormap: HashMap::new(),
            sig: signature.clone(),
            inputs: inputs,
            output_info: info,
            output: vec![],
        }))
    }
}

struct TensorflowExecutionContext {
    graph: Arc<Graph>,
    bundle: Arc<SavedModelBundle>,
    tensormap: HashMap<String, TensorTypes>,
    sig: SignatureDef,
    inputs: Vec<String>,
    output_info: TensorInfo,
    output: Vec<u8>,
}

enum TensorTypes {
    TTU8(TFTensor<u8>),
    TTF16(TFTensor<f32>),
    TTF32(TFTensor<f32>),
    TTI32(TFTensor<i32>),
}

impl BackendExecutionContext for TensorflowExecutionContext {
    fn set_input(&mut self, index: u32, tensor: &Tensor<'_>) -> Result<(), BackendError> {
        // Return an error if the index doesn't exist in the signature.
        if index as usize > self.sig.inputs().len() - 1 {
            return Err(BackendError::InvalidTensorIndex(index as usize));
        }

        let info = self.sig.get_input(&self.inputs[index as usize])?;

        let dim = tensor
            .dimensions
            .as_slice()?
            .iter()
            .map(|d| *d as u64)
            .collect::<Vec<_>>();

        let tfdata = tensor.data.as_slice()?;
        let tfdata_dref = tfdata.deref();
        let data_vec = tfdata_dref.to_vec();

        // Check that the type of the tensor matches the input type
        let matched = match tensor.type_ {
            TensorType::F16 => info.dtype() == tensorflow::DataType::Half,
            TensorType::F32 => info.dtype() == tensorflow::DataType::Float,
            TensorType::U8 => info.dtype() == tensorflow::DataType::UInt8,
            TensorType::I32 => info.dtype() == tensorflow::DataType::Int32,
        };

        if !matched {
            return Err(BackendError::InvalidTensorIndex(index as usize));
        }

        self.save_input_tensor(tensor.type_, index.to_string(), &dim, data_vec.clone());

        Ok(())
    }

    fn compute(&mut self) -> Result<(), BackendError> {
        // Initialize SessionRunArgs for inputs/outputs
        let mut args = SessionRunArgs::new();

        for key in self.tensormap.keys() {
            let key_usize = key.parse::<usize>().unwrap();
            let x_info = self.sig.get_input(self.inputs[key_usize].as_str())?;
            let op_x: Operation = self.graph.operation_by_name_required(&x_info.name().name)?;

            use TensorTypes::*;
            match &self.tensormap[key] {
                TTU8(t) => args.add_feed(&op_x, key_usize as i32, &t),
                TTF16(t) => args.add_feed(&op_x, key_usize as i32, &t),
                TTF32(t) => args.add_feed(&op_x, key_usize as i32, &t),
                TTI32(t) => args.add_feed(&op_x, key_usize as i32, &t),
            }
        }

        // Setup the output
        let op_output = &self
            .graph
            .operation_by_name_required(&self.output_info.name().name)?;
        let token_output: FetchToken = args.request_fetch(&op_output, 0);
        // Run the inference
        self.bundle.session.run(&mut args)?;

        // Save the output for later before the SessionRunArgs go out of scope
        match self.output_info.dtype() {
            tensorflow::DataType::Float | tensorflow::DataType::Half => {
                self.output = sort_results(args.fetch::<f32>(token_output)?);
            }
            tensorflow::DataType::Int32 => {
                self.output = sort_results(args.fetch::<i32>(token_output)?);
            }
            tensorflow::DataType::UInt8 => {
                self.output = sort_results(args.fetch::<u8>(token_output)?);
            }
            _ => {
                return Err(BackendError::UnsupportedOutputPrecision());
            }
        }

        Ok(())
    }

    fn get_output(&mut self, _index: u32, destination: &mut [u8]) -> Result<u32, BackendError> {
        let output = &self.output;
        if output.len() > destination.len() {
            return Err(BackendError::NotEnoughMemory(output.len()));
        }

        destination.copy_from_slice(output);

        Ok(output.len() as u32)
    }
}

impl TensorflowExecutionContext {
    // Save an input tensor to the context
    fn save_input_tensor(
        &mut self,
        tensor_type: TensorType,
        index: String,
        dim: &[u64],
        data: Vec<u8>,
    ) {
        let input_tensor: TensorTypes;

        match tensor_type {
            TensorType::F32 => {
                input_tensor = TensorTypes::TTF32(u8_to_t(TFTensor::<f32>::new(&dim), data))
            }
            TensorType::F16 => {
                input_tensor = TensorTypes::TTF16(u8_to_t(TFTensor::<f32>::new(&dim), data))
            }
            TensorType::U8 => {
                input_tensor = TensorTypes::TTU8(u8_to_t(TFTensor::<u8>::new(&dim), data))
            }
            TensorType::I32 => {
                input_tensor = TensorTypes::TTI32(u8_to_t(TFTensor::<i32>::new(&dim), data))
            }
        }

        self.tensormap.insert(index, input_tensor);
    }
}

impl From<Status> for BackendError {
    fn from(e: Status) -> Self {
        BackendError::BackendAccess(anyhow::Error::new(e))
    }
}

fn sort_results<T: tensorflow::TensorType + std::cmp::PartialOrd + Copy>(
    data: tensorflow::Tensor<T>,
) -> Vec<u8> {
    data.iter()
        .enumerate()
        .fold((0, data[0]), |(idx_max, val_max), (idx, val)| {
            if &val_max > val {
                (idx_max, val_max)
            } else {
                (idx, *val)
            }
        });
    let newdata = &data[..];
    return t_to_u8(newdata).to_owned();
}

// Convert u8 to type T
fn u8_to_t<T: tensorflow::TensorType + std::convert::From<u8>>(
    mut input: TFTensor<T>,
    data: Vec<u8>,
) -> TFTensor<T> {
    for i in 0..data.len() {
        input[i] = data[i].try_into().unwrap();
    }
    return input;
}

// Convert type T to u8
fn t_to_u8<T>(data: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            data.as_ptr() as *const u8,
            data.len() * std::mem::size_of::<T>(),
        )
    }
}
