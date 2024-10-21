use anyhow::{Error, Result};
use image::{DynamicImage, RgbImage};
use std::fs;
use wasi_nn::{self, ExecutionTarget, GraphBuilder, GraphEncoding};

pub fn main() -> Result<(), Error> {
    // Read the model file (Resnet18)
    let model = fs::read("fixture/model.pt")?;
    let graph = GraphBuilder::new(GraphEncoding::Pytorch, ExecutionTarget::CPU)
        .build_from_bytes(&[&model])?;

    let mut context = graph.init_execution_context()?;

    let image = fs::read("fixture/kitten.png")?;
    // Preprocessing. Normalize data based on model requirements https://github.com/onnx/models/tree/main/validated/vision/classification/mobilenet#preprocessing
    let tensor_data = preprocess(
        image.as_slice(),
        224,
        224,
        &[0.485, 0.456, 0.406],
        &[0.229, 0.224, 0.225],
    );
    let precision = wasi_nn::TensorType::F32;
    // Resnet18 model input is NCHW
    let shape = &[1, 3, 224, 224];
    // Set the input tensor. PyTorch models do not use ports, so it is set to 0 here. 
    // Tensors are passed to the model, and the model's forward method processes these tensors.
    context.set_input(0, precision, shape, &tensor_data)?;
    context.compute()?;
    let mut output_buffer = vec![0f32; 1000];
    context.get_output(0, &mut output_buffer[..])?;
    let result = softmax(output_buffer);
    println!(
        "Found results, sorted top 5: {:?}",
        &sort_results(&result)[..5]
    );
    Ok(())
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
    u8_f32_arr
}

fn softmax(output_tensor: Vec<f32>) -> Vec<f32> {
    let max_val = output_tensor
        .iter()
        .cloned()
        .fold(f32::NEG_INFINITY, f32::max);

    // Compute the exponential of each element subtracted by max_val for numerical stability.
    let exps: Vec<f32> = output_tensor.iter().map(|&x| (x - max_val).exp()).collect();

    // Compute the sum of the exponentials.
    let sum_exps: f32 = exps.iter().sum();

    // Normalize each element to get the probabilities.
    exps.iter().map(|&exp| exp / sum_exps).collect()
}

fn sort_results(buffer: &[f32]) -> Vec<InferenceResult> {
    let mut results: Vec<InferenceResult> = buffer
        .iter()
        .enumerate()
        .map(|(c, p)| InferenceResult(c, *p))
        .collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    results
}

#[derive(Debug, PartialEq)]
struct InferenceResult(usize, f32);
