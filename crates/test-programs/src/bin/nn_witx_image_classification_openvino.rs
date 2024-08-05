use anyhow::{Context, Result};
use std::fs;
use test_programs::nn::{sort_results, witx};

pub fn main() -> Result<()> {
    let xml = fs::read("fixture/model.xml")
        .context("the model file to be mapped to the fixture directory")?;
    let weights = fs::read("fixture/model.bin")
        .context("the weights file to be mapped to the fixture directory")?;
    let graph = witx::load(
        &[&xml, &weights],
        witx::GraphEncoding::Openvino,
        witx::ExecutionTarget::CPU,
    )?;
    let tensor = fs::read("fixture/tensor.bgr")
        .context("the tensor file to be mapped to the fixture directory")?;
    let results = witx::classify(graph, tensor)?;
    let top_five = &sort_results(&results)[..5];
    println!("found results, sorted top 5: {top_five:?}");
    Ok(())
}
