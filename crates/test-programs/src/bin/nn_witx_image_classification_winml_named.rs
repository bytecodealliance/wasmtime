use anyhow::{Context, Result};
use std::fs;
use test_programs::nn::{sort_results, witx};

pub fn main() -> Result<()> {
    let graph = witx::load_by_name(
        "mobilenet",
        witx::GraphEncoding::Onnx,
        witx::ExecutionTarget::CPU,
    )?;
    let tensor = fs::read("fixture/tensor.bgr")
        .context("the tensor file to be mapped to the fixture directory")?;
    let results = witx::classify(graph, tensor)?;
    let top_five = &sort_results(&results)[..5];
    println!("found results, sorted top 5: {top_five:?}");
    assert_eq!(top_five[0].class_id(), 284);
    Ok(())
}
