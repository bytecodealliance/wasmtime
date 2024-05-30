use anyhow::Result;
use std::fs;
use test_programs::ml::graph::{self, ExecutionTarget, GraphEncoding};
use test_programs::nn::{classify, sort_results};

pub fn main() -> Result<()> {
    let xml = fs::read("fixture/model.xml")
        .expect("the model file to be mapped to the fixture directory");
    let weights = fs::read("fixture/model.bin")
        .expect("the weights file to be mapped to the fixture directory");
    let graph = graph::load(
        &[xml, weights],
        GraphEncoding::Openvino,
        ExecutionTarget::Cpu,
    )?;
    let tensor = fs::read("fixture/tensor.bgr")
        .expect("the tensor file to be mapped to the fixture directory");
    let results = classify(graph, tensor)?;
    let top_five = &sort_results(&results)[..5];
    println!("found results, sorted top 5: {:?}", top_five);
    Ok(())
}
