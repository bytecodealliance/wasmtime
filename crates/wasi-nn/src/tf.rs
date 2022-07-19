//! Implements the wasi-nn API.
use crate::api::{Backend, BackendError, BackendExecutionContext, BackendGraph};
use crate::witx::types::{ExecutionTarget, GraphBuilderArray, Tensor};
use std::collections::HashMap;
use std::ops::Deref;
use std::path::Path;
use std::str;
use std::sync::Arc;
use tensorflow::DEFAULT_SERVING_SIGNATURE_DEF_KEY;
use tensorflow::{
    FetchToken, Graph, Operation, SavedModelBundle, SessionOptions, SessionRunArgs, Status,
};
use tensorflow::{SignatureDef, Tensor as TFTensor};

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

        // Currently we only support using one output, index == 0
        let out_name = &signature.get_output(outputs[0].as_str())?.name().name;

        Ok(Box::new(TensorflowExecutionContext {
            graph: self.0.clone(),
            bundle: self.1.clone(),
            inputs: HashMap::new(),
            sig: signature.clone(),
            output_name: String::from(out_name),
            output: None,
        }))
    }
}

struct TensorflowExecutionContext {
    graph: Arc<Graph>,
    bundle: Arc<SavedModelBundle>,
    inputs: HashMap<String, TFTensor<f32>>,
    sig: SignatureDef,
    output_name: String,
    output: Option<TFTensor<f32>>,
}

impl BackendExecutionContext for TensorflowExecutionContext {
    fn set_input(&mut self, index: u32, tensor: &Tensor<'_>) -> Result<(), BackendError> {
        let dim = tensor
            .dimensions
            .as_slice()?
            .iter()
            .map(|d| *d as u64)
            .collect::<Vec<_>>();

        let tfdata = tensor.data.as_slice()?;
        let tfdata_dref = tfdata.deref();
        let mut local_tensor = TFTensor::<f32>::new(&dim);

        for i in 0..tfdata_dref.len() {
            local_tensor[i] = tfdata_dref[i] as f32;
        }

        // Save the input to the context
        self.inputs.insert(index.to_string(), local_tensor.clone());

        Ok(())
    }

    fn compute(&mut self) -> Result<(), BackendError> {
        let mut inputs: Vec<String> = vec![];

        // Get the available inputs for this model
        for key in self.sig.inputs().keys() {
            inputs.push(key.clone());
        }

        // Initialize SessionRunArgs for inputs/outputs
        let mut args = SessionRunArgs::new();

        // Check that the saved inputs exist in the signature, and add them to the feed.
        for key in self.inputs.keys() {
            let key_usize = key.parse::<usize>().unwrap();

            if key_usize > self.sig.inputs().len() - 1 {
                return Err(BackendError::InvalidTensorIndex(key_usize));
            }

            let x_info = self.sig.get_input(inputs[key_usize].as_str())?;
            let op_x: Operation = self.graph.operation_by_name_required(&x_info.name().name)?;
            args.add_feed(&op_x, key_usize as i32, &self.inputs[key]);
        }

        // Setup the output
        let op_output = &self.graph.operation_by_name_required(&self.output_name)?;
        let token_output: FetchToken = args.request_fetch(&op_output, 0);
        // Run the inference
        self.bundle.session.run(&mut args)?;
        // Save the output for later
        self.output = Some(args.fetch(token_output)?);

        Ok(())
    }

    fn get_output(&mut self, _index: u32, destination: &mut [u8]) -> Result<u32, BackendError> {
        let output = self.output.as_ref().unwrap();
        if output.len() > destination.len() {
            return Err(BackendError::NotEnoughMemory(output.len()));
        }

        destination.copy_from_slice(f32_to_u8(&output[..]));

        output
            .iter()
            .enumerate()
            .fold((0, output[0]), |(idx_max, val_max), (idx, val)| {
                if &val_max > val {
                    (idx_max, val_max)
                } else {
                    (idx, *val)
                }
            });

        Ok(output.len() as u32)
    }
}

impl From<Status> for BackendError {
    fn from(e: Status) -> Self {
        BackendError::BackendAccess(anyhow::Error::new(e))
    }
}

fn f32_to_u8(data: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}
