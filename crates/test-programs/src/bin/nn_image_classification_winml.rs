use anyhow::Result;
use std::fs;
use std::time::Instant;
use wasi_nn::*;

pub fn main() -> Result<()> {
    // Graph is supposed to be preloaded by `nn-graph` argument. The path ends with "mobilenet".
    let graph =
        wasi_nn::GraphBuilder::new(wasi_nn::GraphEncoding::Onnx, wasi_nn::ExecutionTarget::CPU)
            .build_from_cache("mobilenet")
            .unwrap();

    let mut context = graph.init_execution_context().unwrap();
    println!("Created an execution context.");

    // Convert image to tensor data.
    let tensor_data = fs::read("fixture/kitten.rgb")?;
    context
        .set_input(0, TensorType::F32, &[1, 3, 224, 224], &tensor_data)
        .unwrap();

    // Execute the inference.
    let before_compute = Instant::now();
    context.compute().unwrap();
    println!(
        "Executed graph inference, took {} ms.",
        before_compute.elapsed().as_millis()
    );

    // Retrieve the output.
    let mut output_buffer = vec![0f32; 1000];
    context.get_output(0, &mut output_buffer[..]).unwrap();

    let result = sort_results(&output_buffer);
    println!("Found results, sorted top 5: {:?}", &result[..5]);
    assert_eq!(result[0].0, 284);
    Ok(())
}

// Sort the buffer of probabilities. The graph places the match probability for each class at the
// index for that class (e.g. the probability of class 42 is placed at buffer[42]). Here we convert
// to a wrapping InferenceResult and sort the results. It is unclear why the MobileNet output
// indices are "off by one" but the `.skip(1)` below seems necessary to get results that make sense
// (e.g. 763 = "revolver" vs 762 = "restaurant")
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
