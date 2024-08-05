use anyhow::{Context, Result};
use std::fs;
use test_programs::nn::{sort_results, wit};

pub fn main() -> Result<()> {
    let xml = fs::read("fixture/model.xml")
        .context("the model file to be mapped to the fixture directory")?;
    let weights = fs::read("fixture/model.bin")
        .context("the weights file to be mapped to the fixture directory")?;
    let graph = wit::load(
        &[xml, weights],
        wit::GraphEncoding::Openvino,
        wit::ExecutionTarget::Cpu,
    )?;
    let tensor = fs::read("fixture/tensor.bgr")
        .context("the tensor file to be mapped to the fixture directory")?;
    let results = wit::classify(
        graph,
        ("input", tensor),
        "MobilenetV2/Predictions/Reshape_1",
    )?;
    let top_five = &sort_results(&results)[..5];
    println!("found results, sorted top 5: {top_five:?}");
    Ok(())
}
