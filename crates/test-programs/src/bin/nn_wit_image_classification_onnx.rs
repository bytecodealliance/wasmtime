use anyhow::{Context, Result};
use std::fs;
use test_programs::nn::{sort_results, wit};

pub fn main() -> Result<()> {
    let model = fs::read("fixture/model.onnx")
        .context("the model file to be mapped to the fixture directory")?;
    let graph = wit::load(
        &[model],
        wit::GraphEncoding::Onnx,
        wit::ExecutionTarget::Cpu,
    )?;
    let tensor = fs::read("fixture/000000062808.rgb")
        .context("the tensor file to be mapped to the fixture directory")?;
    let results = wit::classify(graph, ("input", tensor), "output")?;
    let top_five = &sort_results(&results)[..5];
    // 963 is "meat loaf, meatloaf."
    // https://github.com/onnx/models/blob/bec48b6a70e5e9042c0badbaafefe4454e072d08/validated/vision/classification/synset.txt#L963
    assert_eq!(top_five[0].class_id(), 963);
    println!("found results, sorted top 5: {top_five:?}");
    Ok(())
}
