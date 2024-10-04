#![allow(unused_braces)]
use image::ImageReader;
use image::{DynamicImage, RgbImage};
use ndarray::{Array, Dim};
use std::fs;
use std::io::BufRead;

const IMG_PATH: &str = "fixture/images/dog.jpg";

wit_bindgen::generate!({
    path: "../../wit",
    world: "ml",
});

use self::wasi::nn::{
    graph::{Graph, GraphBuilder, load, ExecutionTarget, GraphEncoding},
    tensor::{Tensor, TensorData, TensorDimensions, TensorType},
};

fn main() {
    // Load the ONNX model - SqueezeNet 1.1-7
    // Full details: https://github.com/onnx/models/tree/main/vision/classification/squeezenet
    let model: GraphBuilder = fs::read("fixture/models/squeezenet1.1-7.onnx").unwrap();
    println!("Read ONNX model, size in bytes: {}", model.len());

    let graph = load(&[model], GraphEncoding::Onnx, ExecutionTarget::Cpu).unwrap();
    println!("Loaded graph into wasi-nn");

    let exec_context = Graph::init_execution_context(&graph).unwrap();
    println!("Created wasi-nn execution context.");

    // Load SquezeNet 1000 labels used for classification
    let labels = fs::read("fixture/labels/squeezenet1.1-7.txt").unwrap();
    let class_labels: Vec<String> = labels.lines().map(|line| line.unwrap()).collect();
    println!("Read ONNX Labels, # of labels: {}", class_labels.len());

    // Prepare WASI-NN tensor - Tensor data is always a bytes vector
    let dimensions: TensorDimensions = vec![1, 3, 224, 224];
    let data: TensorData = image_to_tensor(IMG_PATH.to_string(), 224, 224);
    let tensor = Tensor::new(
        &dimensions,
        TensorType::Fp32,
        &data,
    );
    exec_context.set_input("data", tensor).unwrap();
    println!("Set input tensor");

    // Execute the inferencing
    exec_context.compute().unwrap();
    println!("Executed graph inference");

    // Get the inferencing result (bytes) and convert it to f32
    println!("Getting inferencing output");
    let output_data = exec_context.get_output("squeezenet0_flatten0_reshape0").unwrap().data();

    println!("Retrieved output data with length: {}", output_data.len());
    let output_f32 = bytes_to_f32_vec(output_data);

    let output_shape = [1, 1000, 1, 1];
    let output_tensor = Array::from_shape_vec(output_shape, output_f32).unwrap();

    // Post-Processing requirement: compute softmax to inferencing output
    let exp_output = output_tensor.mapv(|x| x.exp());
    let sum_exp_output = exp_output.sum_axis(ndarray::Axis(1));
    let softmax_output = exp_output / &sum_exp_output;

    let mut sorted = softmax_output
        .axis_iter(ndarray::Axis(1))
        .enumerate()
        .into_iter()
        .map(|(i, v)| (i, v[Dim([0, 0, 0])]))
        .collect::<Vec<(_, _)>>();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    for (index, probability) in sorted.iter().take(3) {
        println!(
            "Index: {} - Probability: {}",
            class_labels[*index], probability
        );
    }
}

pub fn bytes_to_f32_vec(data: Vec<u8>) -> Vec<f32> {
    let chunks: Vec<&[u8]> = data.chunks(4).collect();
    let v: Vec<f32> = chunks
        .into_iter()
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect();

    v.into_iter().collect()
}

// Take the image located at 'path', open it, resize it to height x width, and then converts
// the pixel precision to FP32. The resulting BGR pixel vector is then returned.
fn image_to_tensor(path: String, height: u32, width: u32) -> Vec<u8> {
    let pixels = ImageReader::open(path).unwrap().decode().unwrap();
    let dyn_img: DynamicImage = pixels.resize_exact(width, height, image::imageops::Triangle);
    let bgr_img: RgbImage = dyn_img.to_rgb8();

    // Get an array of the pixel values
    let raw_u8_arr: &[u8] = &bgr_img.as_raw()[..];

    // Create an array to hold the f32 value of those pixels
    let bytes_required = raw_u8_arr.len() * 4;
    let mut u8_f32_arr: Vec<u8> = vec![0; bytes_required];

    // Normalizing values for the model
    let mean = [0.485, 0.456, 0.406];
    let std = [0.229, 0.224, 0.225];

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
