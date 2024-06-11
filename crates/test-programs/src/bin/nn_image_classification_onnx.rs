use anyhow::Result;
use std::fs;
use test_programs::nn::{classify, sort_results};
use wasi_nn::{ExecutionTarget, GraphBuilder, GraphEncoding};

pub fn main() -> Result<()> {
    let model = fs::read("fixture/model.onnx")
        .expect("the model file to be mapped to the fixture directory");
    let graph =
        GraphBuilder::new(GraphEncoding::Onnx, ExecutionTarget::CPU).build_from_bytes([&model])?;
    let tensor = fs::read("fixture/000000062808.rgb")
        .expect("the tensor file to be mapped to the fixture directory");
    let results = classify(graph, tensor)?;
    let top_five = &sort_results(&results)[..5];
    // 963 is meat loaf, meatloaf.
    // https://github.com/onnx/models/blob/bec48b6a70e5e9042c0badbaafefe4454e072d08/validated/vision/classification/synset.txt#L963
    assert_eq!(top_five[0].class_id(), 963);
    println!("found results, sorted top 5: {:?}", top_five);
    Ok(())
}
