//! Implements the wasi-nn API.
use crate::api::{Backend, BackendError, BackendExecutionContext, BackendGraph};
use crate::witx::types::{ExecutionTarget, GraphBuilderArray, Tensor, TensorType};
use std::collections::HashMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::Arc;
use tensorflow::{
    FetchToken, Graph, Operation, SavedModelBundle, SessionOptions, SessionRunArgs, Status,
};
use tensorflow::{SignatureDef, Tensor as TFTensor};
use tensorflow::{TensorInfo, DEFAULT_SERVING_SIGNATURE_DEF_KEY};

#[derive(Default)]
pub(crate) struct TensorflowBackend {
    signature: String,
}

impl Backend for TensorflowBackend {
    fn name(&self) -> &str {
        "tensorflow"
    }

    fn load(
        &mut self,
        builders: &GraphBuilderArray<'_>,
        _target: ExecutionTarget,
        map_dirs: &Vec<(String, String)>,
    ) -> Result<Box<dyn BackendGraph>, BackendError> {
        if !map_dirs.is_empty() {
            if builders.len() < 1 {
                return Err(BackendError::InvalidNumberOfBuilders(1, builders.len()).into());
            }
            // Initialize the Tensorflow backend.
            let _retval = tensorflow::library::load().or_else(|e| {
                println!("Error loading the TensorFlow backend: \n {}", e);
                Err(e)
            });

            // Tensorflow wants to read models from a directory. This path here
            // is the guest-side (WebAssembly) version of that path which we map
            // (from `--mapdir` CLI option) to the host-side path with the
            // actual files. If in the future Tensorflow allows loading models
            // from bytes, that would be a better solution (TODO).
            let builders_len = builders.len();
            let builders = builders.as_ptr();
            let guest_map = builders.read()?.as_slice()?;
            let mapped_directory =
                build_path(&guest_map, map_dirs).ok_or(BackendError::MissingMapDir())?;
            let mut tags: Vec<String> = vec![];
            let mut itr: u32 = 1;

            // Get all the user provided options
            while itr < builders_len {
                let opt = builders.add(itr)?.read()?.as_slice()?.to_owned();
                let mut opt_str = str::from_utf8(&opt).ok().unwrap().split(',');

                match opt_str.next().unwrap() {
                    "signature" => {
                        self.signature = opt_str.next().unwrap().to_owned();
                    }
                    "tag" => tags.push(opt_str.next().unwrap().to_owned()),
                    o => {
                        println!("** Unknown Tensorflow option {}, ignoring... **", o);
                    }
                }
                itr += 1;
            }

            // Load the model.
            let mut graph = Graph::new();
            let bundle = SavedModelBundle::load(
                &SessionOptions::new(),
                &tags,
                &mut graph,
                mapped_directory,
            )?;
            return Ok(Box::new(TensorflowGraph(Arc::new(graph), Arc::new(bundle))));
        }
        Err(BackendError::MissingMapDir())
    }
}
/// Map the `guest_path` to its equivalent host path *if* there is a mapping for
/// it in the `map_dirs`.
fn build_path(guest_path: &[u8], map_dirs: &Vec<(String, String)>) -> Option<PathBuf> {
    let guest_path = Path::new(str::from_utf8(guest_path).ok()?);
    for (guest_base, host_base) in map_dirs {
        let host_base = Path::new(host_base);
        // If this is the map_dir we are looking for...
        if guest_path.starts_with(guest_base) {
            let guest_suffix = guest_path.strip_prefix(guest_base).ok()?;
            let host_path: PathBuf = [host_base, guest_suffix].iter().collect();
            // Get the actual full path
            let canon_path = match host_path.canonicalize() {
                Ok(path) => path,
                Err(_) => return None,
            };
            // Check that the guest path isn't trying to get outside of the host path.
            if canon_path.starts_with(&host_base) {
                return Some(host_path);
            } else {
                return None;
            }
        }
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::Builder;
    #[test]
    fn valid_path() {
        let tmp_dir = Builder::new().prefix("build").tempdir().unwrap();
        let tmp_dir2 = Builder::new().prefix("test").tempdir_in(&tmp_dir).unwrap();
        let tmp_dir_str =
            String::from(&tmp_dir.into_path().into_os_string().into_string().unwrap());
        let suffix = tmp_dir2
            .into_path()
            .strip_prefix(&tmp_dir_str)
            .ok()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let guest_test = format!("{}/{}", "fixture".to_owned(), &suffix);
        let map_dirs = vec![("fixture".to_string(), String::from(tmp_dir_str))];
        let result = build_path(guest_test.as_bytes(), &map_dirs);
        assert_eq!(true, result.is_some());
    }

    #[test]
    fn valid_path_dots() {
        let tmp_dir = Builder::new().prefix("build").tempdir().unwrap();
        let tmp_dir2 = Builder::new().prefix("test").tempdir_in(&tmp_dir).unwrap();
        let tmp_dir_str =
            String::from(&tmp_dir.into_path().into_os_string().into_string().unwrap());
        let suffix = format!(
            "{}/{}",
            tmp_dir2
                .into_path()
                .strip_prefix(&tmp_dir_str)
                .ok()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            "..".to_string()
        );
        let guest_test = format!("{}/{}", "fixture".to_owned(), &suffix);
        let map_dirs = vec![("fixture".to_string(), String::from(tmp_dir_str))];
        let result = build_path(guest_test.as_bytes(), &map_dirs);
        assert_eq!(true, result.is_some());
    }

    #[test]
    fn path_escape_attempt() {
        let tmp_dir = Builder::new().prefix("build").tempdir().unwrap();
        let tmp_dir2 = Builder::new().prefix("test").tempdir_in(&tmp_dir).unwrap();
        let tmp_dir_str =
            String::from(&tmp_dir.into_path().into_os_string().into_string().unwrap());
        let suffix = format!(
            "{}/{}",
            tmp_dir2
                .into_path()
                .strip_prefix(&tmp_dir_str)
                .ok()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            "../../".to_string()
        );
        let guest_test = format!("{}/{}", "fixture".to_owned(), &suffix);
        let map_dirs = vec![("fixture".to_string(), String::from(tmp_dir_str))];
        let result = build_path(guest_test.as_bytes(), &map_dirs);
        assert_eq!(false, result.is_some());
    }
}
