use image::{DynamicImage, RgbImage};
use ndarray::Array;
use std::{fs, time::Instant};

pub fn main() {
    // Load model from a file.
    let graph =
        wasi_nn::GraphBuilder::new(wasi_nn::GraphEncoding::Onnx, wasi_nn::ExecutionTarget::CPU)
            .build_from_files(["fixture/mobilenet.onnx"])
            .unwrap();

    let mut context = graph.init_execution_context().unwrap();
    println!("Created an execution context.");

    // Read image from file and convert it to tensor data.
    let image_data = fs::read("fixture/kitten.png").unwrap();

    // Preprocessing. Normalize data based on model requirements https://github.com/onnx/models/tree/main/validated/vision/classification/mobilenet#preprocessing
    let tensor_data = preprocess(
        image_data.as_slice(),
        224,
        224,
        &[0.485, 0.456, 0.406],
        &[0.229, 0.224, 0.225],
    );
    println!("Read input tensor, size in bytes: {}", tensor_data.len());

    context
        .set_input(0, wasi_nn::TensorType::F32, &[1, 3, 224, 224], &tensor_data)
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

    // Postprocessing. Calculating the softmax probability scores.
    let result = postprocess(output_buffer);

    // Load labels for classification
    let labels_file = fs::read("fixture/synset.txt").unwrap();
    let labels_str = String::from_utf8(labels_file).unwrap();
    let labels: Vec<String> = labels_str
        .lines()
        .map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            words[1..].join(" ")
        })
        .collect();

    println!(
        "Found results, sorted top 5: {:?}",
        &sort_results(&result, &labels)[..5]
    )
}

// Sort the buffer of probabilities. The graph places the match probability for each class at the
// index for that class (e.g. the probability of class 42 is placed at buffer[42]). Here we convert
// to a wrapping InferenceResult and sort the results.
fn sort_results(buffer: &[f32], labels: &Vec<String>) -> Vec<InferenceResult> {
    let mut results: Vec<InferenceResult> = buffer
        .iter()
        .enumerate()
        .map(|(c, p)| InferenceResult(labels[c].clone(), *p))
        .collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    results
}

// Resize image to height x width, and then converts the pixel precision to FP32, normalize with
// given mean and std. The resulting RGB pixel vector is then returned.
fn preprocess(image: &[u8], height: u32, width: u32, mean: &[f32], std: &[f32]) -> Vec<u8> {
    let dyn_img: DynamicImage = image::load_from_memory(image).unwrap().resize_exact(
        width,
        height,
        image::imageops::Triangle,
    );
    let rgb_img: RgbImage = dyn_img.to_rgb8();

    // Get an array of the pixel values
    let raw_u8_arr: &[u8] = &rgb_img.as_raw()[..];

    // Create an array to hold the f32 value of those pixels
    let bytes_required = raw_u8_arr.len() * 4;
    let mut u8_f32_arr: Vec<u8> = vec![0; bytes_required];

    // Read the number as a f32 and break it into u8 bytes
    for i in 0..raw_u8_arr.len() {
        let u8_f32: f32 = raw_u8_arr[i] as f32;
        let rgb_iter = i % 3;

        // Normalize the pixel
        let norm_u8_f32: f32 = (u8_f32 / 255.0 - mean[rgb_iter]) / std[rgb_iter];

        // Convert it to u8 bytes and write it with new shape
        let u8_bytes = norm_u8_f32.to_ne_bytes();
        for j in 0..4 {
            u8_f32_arr[(raw_u8_arr.len() * 4 * rgb_iter / 3) + (i / 3) * 4 + j] = u8_bytes[j];
        }
    }

    return u8_f32_arr;
}

fn postprocess(output_tensor: Vec<f32>) -> Vec<f32> {
    // Post-Processing requirement: compute softmax to inferencing output
    let output_shape = [1, 1000, 1, 1];
    let exp_output = Array::from_shape_vec(output_shape, output_tensor)
        .unwrap()
        .mapv(|x| x.exp());
    let sum_exp_output = exp_output.sum_axis(ndarray::Axis(1));
    let softmax_output = exp_output / &sum_exp_output;
    softmax_output.into_raw_vec()
}

pub fn bytes_to_f32_vec(data: Vec<u8>) -> Vec<f32> {
    let chunks: Vec<&[u8]> = data.chunks(4).collect();
    let v: Vec<f32> = chunks
        .into_iter()
        .map(|c| f32::from_ne_bytes(c.try_into().unwrap()))
        .collect();

    v.into_iter().collect()
}

// A wrapper for class ID and match probabilities.
#[derive(Debug, PartialEq)]
struct InferenceResult(String, f32);
