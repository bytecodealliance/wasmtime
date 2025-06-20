use anyhow::{Context, Result};
use std::fs;
use test_programs::nn::{sort_results, wit};

pub fn main() -> Result<()> {
    let graph = wit::load_by_name("fixtures")?;
    let tensor: Vec<u8> = fs::read("fixture/tensor.bgr")
        .context("the tensor file to be mapped to the fixture directory")?;
    let results = wit::classify(graph, ("input", tensor))?;
    let top_five = &sort_results(&results)[..5];
    println!("found results, sorted top 5: {top_five:?}");
    Ok(())
}
