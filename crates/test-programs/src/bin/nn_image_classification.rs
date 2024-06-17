use anyhow::{Context, Result};
use std::fs;
use test_programs::nn::{classify, sort_results};
use wasi_nn::{ExecutionTarget, GraphBuilder, GraphEncoding};

pub fn main() -> Result<()> {
    let xml = fs::read("fixture/model.xml")
        .context("the model file to be mapped to the fixture directory")?;
    let weights = fs::read("fixture/model.bin")
        .context("the weights file to be mapped to the fixture directory")?;
    let graph = GraphBuilder::new(GraphEncoding::Openvino, ExecutionTarget::CPU)
        .build_from_bytes([&xml, &weights])?;
    let tensor = fs::read("fixture/tensor.bgr")
        .context("the tensor file to be mapped to the fixture directory")?;
    let results = classify(graph, tensor)?;
    let top_five = &sort_results(&results)[..5];
    println!("found results, sorted top 5: {:?}", top_five);
    Ok(())
}
