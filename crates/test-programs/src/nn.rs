//! This module attempts to paper over the differences between the two
//! implementations of wasi-nn: the legacy WITX-based version (`mod witx`) and
//! the up-to-date WIT version (`mod wit`). Since the tests are mainly a simple
//! classifier, this exposes a high-level `classify` function to go along with
//! `load`, etc.
//!
//! This module exists solely for convenience--e.g., reduces test duplication.
//! In the future can be safely disposed of or altered as more tests are added.

/// Call `wasi-nn` functions from WebAssembly using the canonical ABI of the
/// component model via WIT-based tooling. Used by `bin/nn_wit_*.rs` tests.
pub mod wit {
    use anyhow::{Result, anyhow};
    use std::time::Instant;

    // Generate the wasi-nn bindings based on the `*.wit` files.
    wit_bindgen::generate!({
        path: "../wasi-nn/wit",
        world: "ml",
        default_bindings_module: "test_programs::ml"
    });
    use self::wasi::nn::errors;
    use self::wasi::nn::graph::{self, Graph};
    pub use self::wasi::nn::graph::{ExecutionTarget, GraphEncoding}; // Used by tests.
    use self::wasi::nn::tensor::{Tensor, TensorType};

    /// Load a wasi-nn graph from a set of bytes.
    pub fn load(
        bytes: &[Vec<u8>],
        encoding: GraphEncoding,
        target: ExecutionTarget,
    ) -> Result<Graph> {
        graph::load(bytes, encoding, target).map_err(err_as_anyhow)
    }

    /// Load a wasi-nn graph by name.
    pub fn load_by_name(name: &str) -> Result<Graph> {
        graph::load_by_name(name).map_err(err_as_anyhow)
    }

    /// Run a wasi-nn inference using a simple classifier model (single input,
    /// single output).
    pub fn classify(graph: Graph, input: (&str, Vec<u8>), output: &str) -> Result<Vec<f32>> {
        let context = graph.init_execution_context().map_err(err_as_anyhow)?;
        println!("[nn] created wasi-nn execution context with ID: {context:?}");

        // Many classifiers have a single input; currently, this test suite also
        // uses tensors of the same shape, though this is not usually the case.
        let tensor = Tensor::new(&vec![1, 3, 224, 224], TensorType::Fp32, &input.1);
        context.set_input(input.0, tensor).map_err(err_as_anyhow)?;
        println!("[nn] set input tensor: {} bytes", input.1.len());

        let before = Instant::now();
        context.compute().map_err(err_as_anyhow)?;
        println!(
            "[nn] executed graph inference in {} ms",
            before.elapsed().as_millis()
        );

        // Many classifiers emit probabilities as floating point values; here we
        // convert the raw bytes to `f32` knowing all models used here use that
        // type.
        let output = context.get_output(output).map_err(err_as_anyhow)?;
        println!(
            "[nn] retrieved output tensor: {} bytes",
            output.data().len()
        );
        let output: Vec<f32> = output
            .data()
            .chunks(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        Ok(output)
    }

    fn err_as_anyhow(e: errors::Error) -> anyhow::Error {
        anyhow!("error: {e:?}")
    }
}

/// Call `wasi-nn` functions from WebAssembly using the legacy WITX-based
/// tooling. This older API has been deprecated for the newer WIT-based API but
/// retained for backwards compatibility testing--i.e., `bin/nn_witx_*.rs`
/// tests.
pub mod witx {
    use anyhow::Result;
    use std::time::Instant;
    pub use wasi_nn::{ExecutionTarget, GraphEncoding};
    use wasi_nn::{Graph, GraphBuilder, TensorType};

    /// Load a wasi-nn graph from a set of bytes.
    pub fn load(
        bytes: &[&[u8]],
        encoding: GraphEncoding,
        target: ExecutionTarget,
    ) -> Result<Graph> {
        Ok(GraphBuilder::new(encoding, target).build_from_bytes(bytes)?)
    }

    /// Load a wasi-nn graph by name.
    pub fn load_by_name(
        name: &str,
        encoding: GraphEncoding,
        target: ExecutionTarget,
    ) -> Result<Graph> {
        Ok(GraphBuilder::new(encoding, target).build_from_cache(name)?)
    }

    /// Run a wasi-nn inference using a simple classifier model (single input,
    /// single output).
    pub fn classify(graph: Graph, tensor: Vec<u8>) -> Result<Vec<f32>> {
        let mut context = graph.init_execution_context()?;
        println!("[nn] created wasi-nn execution context with ID: {context}");

        // Many classifiers have a single input; currently, this test suite also
        // uses tensors of the same shape, though this is not usually the case.
        context.set_input(0, TensorType::F32, &[1, 3, 224, 224], &tensor)?;
        println!("[nn] set input tensor: {} bytes", tensor.len());

        let before = Instant::now();
        context.compute()?;
        println!(
            "[nn] executed graph inference in {} ms",
            before.elapsed().as_millis()
        );

        // Many classifiers emit probabilities as floating point values; here we
        // convert the raw bytes to `f32` knowing all models used here use that
        // type.
        let mut output_buffer = vec![0u8; 1001 * std::mem::size_of::<f32>()];
        let num_bytes = context.get_output(0, &mut output_buffer)?;
        println!("[nn] retrieved output tensor: {num_bytes} bytes");
        let output: Vec<f32> = output_buffer[..num_bytes]
            .chunks(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        Ok(output)
    }
}

/// Sort some classification probabilities.
///
/// Many classification models output a buffer of probabilities for each class,
/// placing the match probability for each class at the index for that class
/// (the probability of class `N` is stored at `probabilities[N]`).
pub fn sort_results(probabilities: &[f32]) -> Vec<InferenceResult> {
    let mut results: Vec<InferenceResult> = probabilities
        .iter()
        .enumerate()
        .map(|(c, p)| InferenceResult(c, *p))
        .collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    results
}

// A wrapper for class ID and match probabilities.
#[derive(Debug, PartialEq)]
pub struct InferenceResult(usize, f32);
impl InferenceResult {
    pub fn class_id(&self) -> usize {
        self.0
    }
    pub fn probability(&self) -> f32 {
        self.1
    }
}
