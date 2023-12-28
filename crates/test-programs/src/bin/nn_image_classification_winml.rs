use anyhow::Result;
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
    let mut tensor_data = image2tensor::convert_image_to_planar_tensor_bytes(
        "fixture/kitten.png",
        224,
        224,
        image2tensor::TensorType::F32,
        image2tensor::ColorOrder::RGB,
    )
    .unwrap();
    // The model requires values in the range of [0, 1].
    scale(&mut tensor_data);
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
    assert_eq!(result[0].0, 284);

    println!("Found results, sorted top 5: {:?}", &result[..5]);
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

// Convert values from [0, 255] to [0, 1].
fn scale(buffer: &mut Vec<u8>) {
    const F32_LEN: usize = 4;
    for i in (0..buffer.len()).step_by(F32_LEN) {
        let mut num = f32::from_ne_bytes(buffer[i..i + F32_LEN].try_into().unwrap());
        num /= 225.0;
        let num_vec = num.to_ne_bytes().to_vec();
        buffer.splice(i..i + F32_LEN, num_vec);
    }
}

// A wrapper for class ID and match probabilities.
#[derive(Debug, PartialEq)]
struct InferenceResult(usize, f32);
