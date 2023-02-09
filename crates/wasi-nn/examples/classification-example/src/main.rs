use std::fs;

wit_bindgen_rust::import!("spec/wasi-nn.wit.md");

pub fn main() {
    let xml = fs::read_to_string("fixture/model.xml").unwrap();
    println!("Read graph XML, first 50 characters: {}", &xml[..50]);

    let weights = fs::read("fixture/model.bin").unwrap();
    println!("Read graph weights, size in bytes: {}", weights.len());

    let graph = wasi_nn::load(
        &[&xml.into_bytes(), &weights],
        wasi_nn::GraphEncoding::Openvino,
        wasi_nn::ExecutionTarget::Cpu,
    )
    .unwrap();
    println!("Loaded graph into wasi-nn with ID: {}", graph.as_raw());

    let context = wasi_nn::init_execution_context(&graph).unwrap();
    println!(
        "Created wasi-nn execution context with ID: {}",
        context.as_raw()
    );

    // Load a tensor that precisely matches the graph input tensor (see
    // `fixture/frozen_inference_graph.xml`).
    let tensor_data = fs::read("fixture/tensor.bgr").unwrap();
    println!("Read input tensor, size in bytes: {}", tensor_data.len());
    let tensor = wasi_nn::TensorParam {
        dimensions: &[1, 3, 224, 224],
        tensor_type: wasi_nn::TensorType::Fp32,
        data: &tensor_data,
    };
    wasi_nn::set_input(&context, 0, tensor).unwrap();

    // Execute the inference.
    wasi_nn::compute(&context).unwrap();
    println!("Executed graph inference");

    // Retrieve the output.
    let output = wasi_nn::get_output(&context, 0).unwrap();
    let data: Vec<u8> = output.data;
    // XXX is this safe wrt alignment?
    let f32s: &[f32] = unsafe {
        std::slice::from_raw_parts(
            data.as_ptr() as *const f32,
            data.len() / std::mem::size_of::<f32>(),
        )
    };
    println!(
        "Found results, sorted top 5: {:?}",
        &sort_results(f32s)[..5]
    )
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
