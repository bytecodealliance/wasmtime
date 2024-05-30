use crate::ml::graph::Graph;
use crate::ml::inference;
use crate::ml::tensor::{Tensor, TensorType};
use anyhow::Result;
use std::time::Instant;

/// Run a wasi-nn inference using a simple classifier model (single input,
/// single output).
pub fn classify(graph: Graph, tensor: Vec<u8>) -> Result<Vec<f32>> {
    let context = inference::init_execution_context(graph)?;
    println!(
        "[nn] created wasi-nn execution context with ID: {}",
        context
    );

    // Many classifiers have a single input; currently, this test suite also
    // uses tensors of the same shape, though this is not usually the case.
    let tensor = Tensor {
        dimensions: vec![1, 3, 224, 224],
        tensor_type: TensorType::Fp32,
        data: tensor,
    };
    inference::set_input(context, 0, &tensor)?;
    println!("[nn] set input tensor: {} bytes", tensor.data.len());

    let before = Instant::now();
    inference::compute(context)?;
    println!(
        "[nn] executed graph inference in {} ms",
        before.elapsed().as_millis()
    );

    // Many classifiers emit probabilities as floating point values; here we
    // convert the raw bytes to `f32` knowing all models used here use that
    // type.
    let output = inference::get_output(context, 0)?;
    println!("[nn] retrieved output tensor: {} bytes", output.len());
    let output: Vec<f32> = output
        .chunks(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    Ok(output)
}

/// Sort some classification probabilities.
///
/// Many classification models output a buffer of probabilities for each class,
/// placing the match probability for each class at the index for that class
/// (the probability of class `N` is stored at `probabilities[N]`).
pub fn sort_results(probabilities: &[f32]) -> Vec<InferenceResult> {
    // It is unclear why the MobileNet output indices are "off by one" but the
    // `.skip(1)` below seems necessary to get results that make sense (e.g. 763
    // = "revolver" vs 762 = "restaurant").
    let mut results: Vec<InferenceResult> = probabilities
        .iter()
        .skip(1)
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
