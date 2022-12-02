//! Implements the wasi-nn API using TensorFlow.

use crate::api::{Backend, BackendError, BackendExecutionContext, BackendGraph};
use crate::witx::types::{ExecutionTarget, GraphBuilderArray, Tensor, TensorType};
use anyhow::anyhow;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::Arc;
use tensorflow::{
    FetchToken, Graph, SavedModelBundle, Session, SessionOptions, SessionRunArgs, SignatureDef,
    Status, Tensor as TFTensor, DEFAULT_SERVING_SIGNATURE_DEF_KEY,
};

#[derive(Default)]
pub(crate) struct TensorflowBackend;

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
            tensorflow::library::load().or_else(|e| {
                println!("Error loading the TensorFlow backend: \n {}", e);
                Err(BackendError::BackendAccess(anyhow!("e")))
            })?;

            // Tensorflow wants to read models from a directory. This path here
            // is the guest-side (WebAssembly) version of that path which we map
            // (from `--mapdir` CLI option) to the host-side path with the
            // actual files. If in the future Tensorflow allows loading models
            // from bytes, that would be a better solution (TODO).
            let builders_len = builders.len();
            let builders = builders.as_ptr();
            let guest_map = builders.read()?.as_slice()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)");
            let mapped_directory =
                build_path(&guest_map, map_dirs).ok_or(BackendError::MissingMapDir())?;
            let mut tags: Vec<String> = vec![];
            let mut signature_str = DEFAULT_SERVING_SIGNATURE_DEF_KEY.to_string();

            if builders_len > 1 {
                // Get the user provided signature.
                let sig_opt = builders.add(1)?.read()?.as_slice()?;
                if sig_opt.is_some() {
                    signature_str = str::from_utf8(&sig_opt.unwrap()).unwrap().to_owned();

                    if builders_len > 2 {
                        // Get all the user provided tags.
                        let mut itr: u32 = 2;
                        while itr < builders_len {
                            let tag_opt = builders.add(itr)?.read()?.as_slice()?;
                            let opt = str::from_utf8(&tag_opt.unwrap()).unwrap().to_owned();
                            tags.push(opt);
                            itr += 1;
                        }
                    }
                }
            }

            // If no tags were provided, try using 'serve' as a default.
            if tags.is_empty() {
                tags.push("serve".to_string());
            }

            // Load the model.
            let mut graph = Graph::new();
            let bundle = SavedModelBundle::load(
                &SessionOptions::new(),
                &tags,
                &mut graph,
                mapped_directory,
            )?;

            // Extract the model signature.
            let signature = bundle
                .meta_graph_def()
                .get_signature(&signature_str)?
                .clone();

            return Ok(Box::new(TensorflowGraph {
                graph: Arc::new(graph),
                session: Arc::new(bundle.session),
                signature: Arc::new(signature),
            }));
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
        // If this is the mapped directory we are looking for...
        if guest_path.starts_with(guest_base) {
            // ...then map the guest path to its host equivalent.
            let guest_suffix = guest_path.strip_prefix(guest_base).ok()?;
            let host_path: PathBuf = [host_base, guest_suffix].iter().collect();
            // Now canonicalize the host path to check that the guest path
            // has not escaped the host's base path.
            let canon_path = host_path.canonicalize().ok()?;
            return canon_path.starts_with(&host_base).then_some(host_path);
        }
    }
    None
}

struct TensorflowGraph {
    graph: Arc<Graph>,
    session: Arc<Session>,
    signature: Arc<SignatureDef>,
}

impl<'a> BackendGraph for TensorflowGraph {
    fn init_execution_context(&mut self) -> Result<Box<dyn BackendExecutionContext>, BackendError> {
        Ok(Box::new(TensorflowExecutionContext {
            graph: self.graph.clone(),
            session: self.session.clone(),
            signature: self.signature.clone(),
            tensors: Vec::new(),
            args: SessionRunArgs::new(),
            output_tokens: Vec::new(),
        }))
    }
}

// An enum wrapper around each of the Tensor types so we can store input
// Tensors of different types in the same vector.
enum TensorTypes {
    TTU8(TFTensor<u8>),
    TTF16(TFTensor<f32>),
    TTF32(TFTensor<f32>),
    TTI32(TFTensor<i32>),
}

struct TensorflowExecutionContext<'a> {
    graph: Arc<Graph>,
    session: Arc<Session>,
    signature: Arc<SignatureDef>,
    tensors: Vec<TensorTypes>,
    args: SessionRunArgs<'a>,
    output_tokens: Vec<(FetchToken, tensorflow::DataType)>,
}

impl<'a> BackendExecutionContext for TensorflowExecutionContext<'a> {
    fn set_input(&mut self, index: u32, tensor: &Tensor<'_>) -> Result<(), BackendError> {
        // Return an error if the index doesn't exist in the signature.
        if index as usize > self.signature.inputs().len() - 1 {
            return Err(BackendError::InvalidTensorIndex(index as usize));
        }

        // Sort the input keys alphabetically so that we know we always index
        // into the same key. (Note that TF's `HashMap::keys()` is returned in
        // arbitrary order).
        let mut input_keys: Vec<String> = self.signature.inputs().keys().cloned().collect();
        input_keys.sort();
        let input_key = &input_keys[index as usize];

        // Check that the tensor data type provided matches the one in the model.
        let tensor_info = self.signature.get_input(input_key)?;
        match_tensor_type(index as usize, tensor_info.dtype(), tensor.type_)?;

        // Now, figure out what TF operation to bind this tensor to.
        let operation = self
            .graph
            .operation_by_name_required(&tensor_info.name().name)?;

        // Convert the dimensions to `u64`s.
        let dims = tensor
            .dimensions
            .as_slice()?
            .expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)")
            .iter()
            .map(|d| *d as u64)
            .collect::<Vec<_>>();

        // Copy the tensor bytes to the Tensorflow container. We pretend the
        // tensor has byte elements (though it may contain elements of any
        // `TensorType`) because we expect the user to provide the tensor in the
        // exact, compatible byte format for Tensorflow. Ideally we would avoid
        // the copy here and just point to the original bytes (TODO: investigate
        // unsafely using `as_mut_ptr`).
        self.tensors.push(match tensor.type_ {
            TensorType::F32 => TensorTypes::TTF32(TFTensor::<f32>::new(&dims)),
            TensorType::F16 => TensorTypes::TTF16(TFTensor::<f32>::new(&dims)),
            TensorType::U8 => TensorTypes::TTU8(TFTensor::<u8>::new(&dims)),
            TensorType::I32 => TensorTypes::TTI32(TFTensor::<i32>::new(&dims)),
        });

        // Assign the tensor to the session arguments. The `add_feed`
        // documentation says that because most operations have only one output
        // (and presumably one input), so the input index is likely 0. Note that
        // we need to do some awkward hoop-jumping here:
        // - in order to maintain the lifetime of `SessionRunArgs`, which
        //   borrows the tensor data, we copy the tensor data into our `Self`
        //   above (otherwise we cannot guarantee that the borrowed data will be
        //   there when we actually need it, in `compute`).
        // - but we also must fit within the Wiggle-generated
        //   `BackendExecutionContext` trait, which says this function must take
        //   `&mut self`. So we pretend that `&mut self` lives as long as, well,
        //   itself (`'a`) using `transmute` and use our new `self_` to borrow
        //   the tensor data we copied to `Self`.
        let self_ = unsafe { std::mem::transmute::<&mut Self, &'a mut Self>(self) };
        let tensor_ref = self_.tensors.last_mut().unwrap();
        let data = tensor.data.as_slice()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)");

        // Pack the tensor with the data. If its not a u8 tensor, the data needs to be converted.
        use TensorTypes::*;
        match tensor_ref {
            TTU8(t) => {
                t.clone_from_slice(&data);
                self_.args.add_feed(&operation, 0, t);
            }
            TTF32(t) | TTF16(t) => {
                for i in 0..t.len() {
                    let jmp = i * std::mem::size_of::<f32>();
                    let to_t = [data[jmp], data[jmp + 1], data[jmp + 2], data[jmp + 3]];
                    t[i] = f32::from_ne_bytes(to_t);
                }
                self_.args.add_feed(&operation, 0, t);
            }
            TTI32(t) => {
                for i in 0..t.len() {
                    let jmp = i * std::mem::size_of::<i32>();
                    let to_t = [data[jmp], data[jmp + 1], data[jmp + 2], data[jmp + 3]];
                    t[i] = i32::from_ne_bytes(to_t);
                }
                self_.args.add_feed(&operation, 0, t);
            }
        };
        Ok(())
    }

    fn compute(&mut self) -> Result<(), BackendError> {
        // The output requests must be made before calling session.run, or it will fail.
        // Because we don't know which results the user will want to access,
        // we need to save all the output tokens for later.

        // Reset tokens
        self.output_tokens.clear();

        // Sort the output keys alphabetically so that we know we always index
        // into the same key. (Note that TF's `HashMap::keys()` is returned in
        // arbitrary order).
        let mut output_keys: Vec<String> = self.signature.outputs().keys().cloned().collect();
        output_keys.sort();

        for i in 0..output_keys.len() {
            let output_key = &output_keys[i as usize];
            let output_tensor_info = self.signature.get_output(output_key)?;
            let out_operation = self
                .graph
                .operation_by_name_required(&output_tensor_info.name().name)?;

            // Save the output token.
            self.output_tokens.push((
                self.args.request_fetch(&out_operation, i as i32),
                output_tensor_info.dtype(),
            ));
        }

        self.session.run(&mut self.args)?;
        Ok(())
    }

    fn get_output(&mut self, index: u32, destination: &mut [u8]) -> Result<u32, BackendError> {
        let token_tuple = self.output_tokens[index as usize];

        let results = match token_tuple.1 {
            tensorflow::DataType::UInt8 => {
                t_to_u8_copy(&self.args.fetch::<u8>(token_tuple.0)?, destination)
            }
            tensorflow::DataType::Half | tensorflow::DataType::Float => {
                t_to_u8_copy(&self.args.fetch::<f32>(token_tuple.0)?, destination)
            }
            tensorflow::DataType::Int32 => {
                t_to_u8_copy(&self.args.fetch::<i32>(token_tuple.0)?, destination)
            }
            _ => Err(BackendError::UnsupportedOutputPrecision()),
        };

        // The inference has been completed, reset the SessionRunArgs in preperation for the next one.
        self.args = SessionRunArgs::new();

        if results.is_ok() {
            Ok(destination.len() as u32)
        } else {
            Err(results.unwrap_err())
        }
    }
}

/// Check that the data type of the user-provided tensor matches the one
/// expected by Tensorflow.
fn match_tensor_type(
    index: usize,
    expected: tensorflow::DataType,
    provided: TensorType,
) -> Result<(), BackendError> {
    if let Some(expected) = convert_tensor_type(expected) {
        if expected != provided {
            let expected = format!("{:?}", expected);
            let provided = format!("{:?}", provided);
            return Err(BackendError::InvalidTensorType(index, expected, provided));
        }
    } else {
        let expected = expected.to_string();
        let provided = format!("{:?}", provided);
        return Err(BackendError::InvalidTensorType(index, expected, provided));
    }
    Ok(())
}

/// Convert the Tensorflow data type to its wasi-nn type, if possible.
fn convert_tensor_type(tensor_type: tensorflow::DataType) -> Option<TensorType> {
    match tensor_type {
        tensorflow::DataType::UInt8 => Some(TensorType::U8),
        tensorflow::DataType::Half => Some(TensorType::F16),
        tensorflow::DataType::Int32 => Some(TensorType::I32),
        tensorflow::DataType::Float => Some(TensorType::F32),
        _ => None,
    }
}

impl From<Status> for BackendError {
    fn from(e: Status) -> Self {
        BackendError::BackendAccess(anyhow::Error::new(e))
    }
}

// Convert type T to u8
fn t_to_u8_copy<T>(data: &[T], destination: &mut [u8]) -> Result<(), BackendError> {
    unsafe {
        let tmpu8 = std::slice::from_raw_parts(
            data.as_ptr() as *const u8,
            data.len() * std::mem::size_of::<T>(),
        );

        if tmpu8.len() > destination.len() {
            Err(BackendError::NotEnoughMemory(destination.len()))
        } else {
            destination.copy_from_slice(tmpu8);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{Builder, TempDir};

    fn create_temp_dir<P: AsRef<Path>>(p: P) -> (TempDir, PathBuf) {
        let parent_dir = Builder::new().prefix("wasi-nn-tests").tempdir().unwrap();
        let child_dir = parent_dir.path().join(p);
        std::fs::create_dir_all(&child_dir).unwrap();
        (parent_dir, child_dir)
    }

    #[test]
    fn valid_path() {
        let (_tmp_dir, foo_bar_dir) = create_temp_dir("foo/bar");
        let foo_dir = foo_bar_dir.parent().unwrap();
        let map_dirs = vec![("/baz".to_string(), foo_dir.to_string_lossy().to_string())];

        // Map `/baz/bar` to `<host path>/foo/bar`.
        let result = build_path(b"/baz/bar", &map_dirs);
        assert!(result.is_some());
    }

    #[test]
    fn valid_path_with_parent_dots() {
        let (_tmp_dir, foo_bar_dir) = create_temp_dir("foo/bar");
        let foo_dir = foo_bar_dir.parent().unwrap();
        let map_dirs = vec![("/baz".to_string(), foo_dir.to_string_lossy().to_string())];

        // Map `/baz/bar/..` to `<host path>/foo`.
        let result = build_path(b"/baz/bar", &map_dirs);
        assert!(result.is_some());
    }

    #[test]
    fn invalid_path_escape_attempt() {
        let (_tmp_dir, foo_bar_dir) = create_temp_dir("foo/bar");
        let foo_dir = foo_bar_dir.parent().unwrap();
        let map_dirs = vec![("/baz".to_string(), foo_dir.to_string_lossy().to_string())];

        // It is invalid to map `/baz/..` because it would escape the mapping.
        let result = build_path(b"/baz/..", &map_dirs);
        assert!(result.is_none());
    }
}
