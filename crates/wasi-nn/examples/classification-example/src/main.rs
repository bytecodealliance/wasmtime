use image::{DynamicImage};
use image::io::Reader;
use std::convert::TryInto;
use std::fs;
use wasi_nn;

// TODO change this to wasi_nn::GRAPH_ENCODING_TENSORFLOW once wasi-nn is updated.
// const GRAPH_ENCODING_TENSORFLOW: u8 = 1;

pub fn main() {
    match env!("BACKEND") {
        "openvino" => {
            execute(wasi_nn::GRAPH_ENCODING_OPENVINO, &[1, 3, 224, 224], vec![0f32; 1001]);
        },
        "tensorflow" => {
            execute(wasi_nn::GRAPH_ENCODING_TENSORFLOW, &[1, 224, 224, 3], vec![0f32; 1001]);
        },
        _ => {
            println!("Unknown backend, exiting...");
            return();
        }
    }
}

fn execute(backend: wasi_nn::GraphEncoding, dimensions: &[u32], mut output_buffer: Vec<f32>) {
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
    let tensor_data = image_to_tensor("fixture/train.jpg".to_string(), dimensions, backend);
    println!("Read input tensor, size in bytes: {}", tensor_data.len());

    let tensor = wasi_nn::Tensor {
                    dimensions: dimensions,
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

fn create_gba (backend: u8) ->Vec<Vec<u8>>  {
    let result: Vec<Vec<u8>> = match backend {
        wasi_nn::GRAPH_ENCODING_OPENVINO => {
            let xml = fs::read_to_string("fixture/model.xml").unwrap();
            let weights = fs::read("fixture/model.bin").unwrap();
            Vec::from([xml.into_bytes(), weights])
        },
        wasi_nn::GRAPH_ENCODING_TENSORFLOW => {
            let model_path: String = env!("MAPDIR").to_string();
            Vec::from([model_path.into_bytes(),
                        "signature,serving_default".to_owned().into_bytes(),
                        "tag,serve".to_owned().into_bytes(),
                        ])
        },
        _ => {
            println!("Unknown backend {}", backend);
            vec![]
        }

    };
    return result;
}

// Sort the buffer of probabilities. The graph places the match probability for each class at the
// index for that class (e.g. the probability of class 42 is placed at buffer[42]). Here we convert
// to a wrapping InferenceResult and sort the results.
fn sort_results(buffer: &[f32], backend: u8) -> Vec<InferenceResult> {
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
    results
}

// A wrapper for class ID and match probabilities.
#[derive(Debug, PartialEq)]
struct InferenceResult(usize, f32);

fn image_to_tensor(path: String, dimensions: &[u32], backend: u8) -> Vec<u8> {
    let result: Vec<u8> = match backend {
        wasi_nn::GRAPH_ENCODING_OPENVINO => {
            let pixels = Reader::open(path).unwrap().decode().unwrap();
            let dyn_img: DynamicImage = pixels.resize_exact(dimensions[2], dimensions[3], image::imageops::Triangle);
            let bgr_img = dyn_img.to_bgr8();
            // Get an array of the pixel values
            let raw_u8_arr: &[u8] = &bgr_img.as_raw()[..];
            // Create an array to hold the f32 value of those pixels
            let bytes_required = raw_u8_arr.len() * 4;
            let mut u8_f32_arr:Vec<u8> = vec![0; bytes_required];

            for i in 0..raw_u8_arr.len()  {
                // Read the number as a f32 and break it into u8 bytes
                let u8_f32: f32 = raw_u8_arr[i] as f32;
                let u8_bytes = u8_f32.to_ne_bytes();

                for j in 0..4 {
                    u8_f32_arr[(i * 4) + j] = u8_bytes[j];
                }
            }
            u8_f32_arr
        },
        wasi_nn::GRAPH_ENCODING_TENSORFLOW => {
            let pixels = Reader::open(path).unwrap().decode().unwrap();
            let dyn_img: DynamicImage = pixels.resize_exact(dimensions[1], dimensions[2], image::imageops::Triangle);
            let bgr_img = dyn_img.to_rgb8();
            // Get an array of the pixel values
            let raw_u8_arr: &[u8] = &bgr_img.as_raw()[..];
            // Create an array to hold the f32 value of those pixels
            let mut u8_f32_arr:Vec<u8> = vec![0; raw_u8_arr.len()];

            for i in 0..raw_u8_arr.len() {
                u8_f32_arr[i] = raw_u8_arr[i];
            }

            u8_f32_arr
        },
        _ => {
            println!("Unknown backend {}", backend);
            vec![]
        }
    };
    return result;
}
