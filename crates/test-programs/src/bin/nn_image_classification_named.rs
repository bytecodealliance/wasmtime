use anyhow::Result;
use std::fs;
use wasi_nn::*;

pub fn main() -> Result<()> {
    let graph = GraphBuilder::new(GraphEncoding::Openvino, ExecutionTarget::CPU)
        .build_from_cache("mobilenet")?;
    println!("Loaded a graph: {:?}", graph);

    let mut context = graph.init_execution_context()?;
    println!("Created an execution context: {:?}", context);

    // Load a tensor that precisely matches the graph input tensor (see
    // `fixture/frozen_inference_graph.xml`).
    let tensor_data = fs::read("fixture/tensor.bgr")?;
    println!("Read input tensor, size in bytes: {}", tensor_data.len());
    context.set_input(0, TensorType::F32, &[1, 3, 224, 224], &tensor_data)?;

    // Execute the inference.
    context.compute()?;
    println!("Executed graph inference");

    // Retrieve the output.
    let mut output_buffer = vec![0f32; 1001];
    context.get_output(0, &mut output_buffer[..])?;

    println!(
        "Found results, sorted top 5: {:?}",
        &sort_results(&output_buffer)[..5]
    );
    Ok(())
}

// Sort the buffer of probabilities. The graph places the match probability for
// each class at the index for that class (e.g. the probability of class 42 is
// placed at buffer[42]). Here we convert to a wrapping InferenceResult and sort
// the results. It is unclear why the MobileNet output indices are "off by one"
// but the `.skip(1)` below seems necessary to get results that make sense (e.g.
// 763 = "revolver" vs 762 = "restaurant").
fn sort_results(buffer: &[f32]) -> Vec<InferenceResult> {
    let mut results: Vec<InferenceResult> = buffer
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
struct InferenceResult(usize, f32);
