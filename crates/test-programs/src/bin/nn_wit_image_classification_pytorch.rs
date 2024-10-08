use anyhow::{Context, Result};
use std::fs;
use test_programs::nn::{sort_results, wit};

pub fn main() -> Result<()> {
    let model = fs::read("fixture/model.pt")
        .context("the model file to be mapped to the fixture directory")?;
    let graph = wit::load(
        &[model],
        wit::GraphEncoding::Pytorch,
        wit::ExecutionTarget::Cpu,
    )?;
    let tensor = fs::read("fixture/kitten.tensor")
        .context("the tensor file to be mapped to the fixture directory")?;
    let output_buffer = wit::classify(graph, ("input", tensor), "output")?;
    let result = softmax(output_buffer);
    let top_five = &sort_results(&result)[..5];
    assert_eq!(top_five[0].class_id(), 281);
    println!("found results, sorted top 5: {top_five:?}");
    Ok(())
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
