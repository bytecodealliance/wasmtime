use anyhow::Result;
use std::fs;
use test_programs::nn::{classify, sort_results};
use wasi_nn::{ExecutionTarget, GraphBuilder, GraphEncoding};

pub fn main() -> Result<()> {
    let graph = GraphBuilder::new(GraphEncoding::Openvino, ExecutionTarget::CPU)
        .build_from_cache("fixtures")?;
    let tensor = fs::read("fixture/tensor.bgr")
        .expect("the tensor file to be mapped to the fixture directory");
    let results = classify(graph, tensor)?;
    let top_five = &sort_results(&results)[..5];
    println!("found results, sorted top 5: {:?}", top_five);
    Ok(())
}
