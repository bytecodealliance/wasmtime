//! Implements the wasi-nn API.
use crate::api::{Backend, BackendError, BackendExecutionContext, BackendGraph};
use crate::witx::types::{ExecutionTarget, GraphBuilderArray, Tensor};
use std::ops::Deref;
use std::path::Path;
use std::str;
use std::sync::Arc;
// use tensorflow::library;
use tensorflow::Tensor as TFTensor;
use tensorflow::DEFAULT_SERVING_SIGNATURE_DEF_KEY;
use tensorflow::{
    FetchToken, Graph, Operation, SavedModelBundle, SessionOptions, SessionRunArgs, Status,
};

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
        if map_dir.as_ref().is_some() {
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
                let unmap = map_dir.as_ref().unwrap();
                for i in 0..unmap.len() {
                    if unmap[i].0 == guest_map_str {
                        //Append the stored mapdir path with the user path.
                        let full_path = std::fs::canonicalize(
                            Path::new(&unmap[i].1.clone()).join(Path::new(exp_str)),
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
                        return Ok(Box::new(TensorflowGraph(graph, Arc::new(bundle))));
                    }
                }
            }
        }

        Err(BackendError::MissingMapDir())
    }
}

struct TensorflowGraph(Graph, Arc<SavedModelBundle>);

impl<'a> BackendGraph for TensorflowGraph {
    fn init_execution_context(&mut self) -> Result<Box<dyn BackendExecutionContext>, BackendError> {
        let signature = self
            .1
            .meta_graph_def()
            .get_signature(DEFAULT_SERVING_SIGNATURE_DEF_KEY)?;

        let x_info = signature.get_input("input_1")?;
        let op_x: Operation = self.0.operation_by_name_required(&x_info.name().name)?;
        let output_info = signature.get_output("Predictions")?;
        let op_output = self
            .0
            .operation_by_name_required(&output_info.name().name)?;

        Ok(Box::new(TensorflowExecutionContext {
            op_x: op_x,
            op_output: op_output,
            bundle: self.1.clone(),
            token_output: None,
            tensor: None,
            output: None,
        }))
    }
}

struct TensorflowExecutionContext {
    op_x: Operation,
    op_output: Operation,
    bundle: Arc<SavedModelBundle>,
    token_output: Option<FetchToken>,
    tensor: Option<TFTensor<f32>>,
    output: Option<TFTensor<f32>>,
}

impl BackendExecutionContext for TensorflowExecutionContext {
    fn set_input(&mut self, _index: u32, tensor: &Tensor<'_>) -> Result<(), BackendError> {
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

        self.tensor = Some(local_tensor);
        Ok(())
    }

    fn compute(&mut self) -> Result<(), BackendError> {
        let mut args: SessionRunArgs = SessionRunArgs::new();
        args.add_feed(&self.op_x, 0, &self.tensor.as_ref().unwrap());
        self.token_output = Some(args.request_fetch(&self.op_output, 0));
        self.bundle.session.run(&mut args)?;
        self.output = Some(args.fetch(self.token_output.unwrap())?);
        Ok(())
    }

    fn get_output(&mut self, _index: u32, destination: &mut [u8]) -> Result<u32, BackendError> {
        // Calculate argmax of the output
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
