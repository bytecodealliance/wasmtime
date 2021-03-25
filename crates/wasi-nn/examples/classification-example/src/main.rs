use std::convert::TryInto;
use std::fs;
use wasi_nn;

pub fn main() {
    let xml = fs::read_to_string("fixture/alexnet.xml").unwrap();
    println!("Read graph XML, first 50 characters: {}", &xml[..50]);

    let weights = fs::read("fixture/alexnet.bin").unwrap();
    println!("Read graph weights, size in bytes: {}", weights.len());

    let graph = unsafe {
        wasi_nn::load(
            &[&xml.into_bytes(), &weights],
            wasi_nn::GRAPH_ENCODING_OPENVINO,
            wasi_nn::EXECUTION_TARGET_CPU,
        )
        .unwrap()
    };
    println!("Loaded graph into wasi-nn with ID: {}", graph);

    let context = unsafe { wasi_nn::init_execution_context(graph).unwrap() };
    println!("Created wasi-nn execution context with ID: {}", context);

    // Load a tensor that precisely matches the graph input tensor (see
    // `fixture/frozen_inference_graph.xml`).
    let tensor_data = fs::read("fixture/tensor-1x3x227x227-f32.bgr").unwrap();
    println!("Read input tensor, size in bytes: {}", tensor_data.len());
    let tensor = wasi_nn::Tensor {
        dimensions: &[1, 3, 227, 227],
        r#type: wasi_nn::TENSOR_TYPE_F32,
        data: &tensor_data,
    };
    unsafe {
        wasi_nn::set_input(context, 0, tensor).unwrap();
    }

    // Execute the inference.
    unsafe {
        wasi_nn::compute(context).unwrap();
    }
    println!("Executed graph inference");

    // Retrieve the output.
    let mut output_buffer = vec![0f32; 1000];
    unsafe {
        wasi_nn::get_output(
            context,
            0,
            &mut output_buffer[..] as *mut [f32] as *mut u8,
            (output_buffer.len() * 4).try_into().unwrap(),
        )
        .unwrap();
    }
    println!(
        "Found results, sorted top 5: {:?}",
        &sort_results(&output_buffer)[..5]
    )
}

// Sort the buffer of probabilities. The graph places the match probability for each class at the
// index for that class (e.g. the probability of class 42 is placed at buffer[42]). Here we convert
// to a wrapping InferenceResult and sort the results.
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
