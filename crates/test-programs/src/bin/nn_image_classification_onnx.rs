use anyhow::Result;
use std::fs;
use wasi_nn::*;

pub fn main() -> Result<()> {
    let model = fs::read("fixture/model.onnx").unwrap();
    println!("[ONNX] Read model, size in bytes: {}", model.len());

    let graph =
        GraphBuilder::new(GraphEncoding::Onnx, ExecutionTarget::CPU).build_from_bytes([&model])?;

    let mut context = graph.init_execution_context()?;
    println!(
        "[ONNX] Created wasi-nn execution context with ID: {}",
        context
    );

    // Prepare WASI-NN tensor - Tensor data is always a bytes vector
    // Load a tensor that precisely matches the graph input tensor
    let data = fs::read("fixture/tensor.bgr").unwrap();
    println!("[ONNX] Read input tensor, size in bytes: {}", data.len());
    context.set_input(0, wasi_nn::TensorType::F32, &[1, 3, 224, 224], &data)?;

    // Execute the inferencing
    context.compute()?;
    println!("[ONNX] Executed graph inference");

    // Retrieve the output.
    let mut output_buffer = vec![0f32; 1000];
    context.get_output(0, &mut output_buffer[..])?;
    println!(
        "[ONNX] Found results, sorted top 5: {:?}",
        &sort_results(&output_buffer)[..5]
    );

    Ok(())
}

// Sort the buffer of probabilities. The graph places the match probability for
// each class at the index for that class (e.g. the probability of class 42 is
// placed at buffer[42]). Here we convert to a wrapping InferenceResult and sort
// the results.
fn sort_results(buffer: &[f32]) -> Vec<InferenceResult> {
    let mut results: Vec<InferenceResult> = buffer
        .iter()
        .enumerate()
        .map(|(c, p)| InferenceResult(c, *p))
        .collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    results
}

// A wrapper for class ID and match probabilities.
#[derive(Debug, PartialEq)]
struct InferenceResult(usize, f32);
