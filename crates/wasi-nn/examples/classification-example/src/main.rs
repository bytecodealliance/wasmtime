use image2tensor::*;
use std::convert::TryInto;
use std::fs;
use wasi_nn;
mod imagenet_classes;

pub fn main() {
    match env!("BACKEND") {
        "openvino" => {
            execute(wasi_nn::GRAPH_ENCODING_OPENVINO, &[1, 3, 224, 224], vec![0f32; 1001], TensorType::F32, ColorOrder::BGR);
        },
        "tensorflow" => {
            execute(wasi_nn::GRAPH_ENCODING_TENSORFLOW, &[1, 224, 224, 3], vec![0f32; 1000], TensorType::F32, ColorOrder::RGB);
        },
        _ => {
            println!("Unknown backend, exiting...");
            return();
        }
    }
}

fn execute(backend: wasi_nn::GraphEncoding, dimensions: &[u32], mut output_buffer: Vec<f32>, precision: TensorType, color_order: ColorOrder) {
    let mut gba_r: Vec<&[u8]> = vec![];
    let gba = create_gba(backend);

    for i in 0..gba.len() {
        gba_r.push(gba[i].as_slice());
    }

    let graph = unsafe {
        wasi_nn::load(
            &gba_r,
            backend,
            wasi_nn::EXECUTION_TARGET_CPU,
        )
        .unwrap()
    };

    println!("Loaded graph into wasi-nn with ID: {}", graph);
    let context = unsafe { wasi_nn::init_execution_context(graph).unwrap() };
    println!("Created wasi-nn execution context with ID: {}", context);

    // Load a tensor that precisely matches the graph input tensor
    let tensor_data = convert_image_to_bytes("fixture/train.jpg", 224, 224, precision, color_order).or_else(|e| {
        Err(e)
    }).unwrap();

    println!("Input tensor size in bytes: {}", tensor_data.len());
    let tensor = wasi_nn::Tensor {
                    dimensions: dimensions,
                    type_: wasi_nn::TENSOR_TYPE_F32,
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
        &sort_results(&output_buffer, backend)[..5]
    )
}

fn create_gba (backend: wasi_nn::GraphEncoding) ->Vec<Vec<u8>>  {
    let result: Vec<Vec<u8>> = match backend {
        wasi_nn::GRAPH_ENCODING_OPENVINO => {
            let xml = fs::read_to_string("fixture/model.xml").unwrap();
            let weights = fs::read("fixture/model.bin").unwrap();
            Vec::from([xml.into_bytes(), weights])
        },
        wasi_nn::GRAPH_ENCODING_TENSORFLOW => {
            let model_path: String = env!("MAPDIR").to_string();
            Vec::from([model_path.into_bytes(),
                        "serving_default".to_owned().into_bytes(),
                        "serve".to_owned().into_bytes(),
                        ])
        },
        _ => {
            println!("Unknown backend {:?}", backend);
            vec![]
        }

    };
    return result;
}

// Sort the buffer of probabilities. The graph places the match probability for each class at the
// index for that class (e.g. the probability of class 42 is placed at buffer[42]). Here we convert
// to a wrapping InferenceResult and sort the results.
fn sort_results(buffer: &[f32], backend: wasi_nn::GraphEncoding) -> Vec<InferenceResult> {
    let skipval = match backend {
        wasi_nn::GRAPH_ENCODING_OPENVINO => { 1 },
        _ => { 0 }
    };

    let mut results: Vec<InferenceResult> = buffer
        .iter()
        .skip(skipval)
        .enumerate()
        .map(|(c, p)| InferenceResult(c, *p))
        .collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for i in 0..5 {
        println!("{}.) {} = ({:?})", i + 1, imagenet_classes::IMAGENET_CLASSES[results[i].0], results[i]);
    }
    results
}


// A wrapper for class ID and match probabilities.
#[derive(Debug, PartialEq)]
struct InferenceResult(usize, f32);
