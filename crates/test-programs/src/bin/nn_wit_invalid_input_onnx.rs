//! Send an input tensor with the wrong byte length to the ONNX backend.
//! Check that the backend returns an error and does not trap.

use anyhow::{Context, Result, anyhow, ensure};
use std::fs;
use test_programs::nn::wit::{self, Tensor, TensorType};

/// The byte length of the `[1, 3, 224, 224]` `f32` input of MobileNet.
const INPUT_BYTES: usize = 1 * 3 * 224 * 224 * 4;

pub fn main() -> Result<()> {
    let model = fs::read("fixture/model.onnx")
        .context("the model file to be mapped to the fixture directory")?;
    let graph = wit::load(
        &[model],
        wit::GraphEncoding::Onnx,
        wit::ExecutionTarget::Cpu,
    )?;
    let context = graph
        .init_execution_context()
        .map_err(|e| anyhow!("{e:?}"))?;

    for len in [INPUT_BYTES - 1, INPUT_BYTES + 1] {
        let tensor = Tensor::new(&[1, 3, 224, 224], TensorType::Fp32, &vec![0u8; len]);
        let result = context.compute(vec![("input".to_string(), tensor)]);
        ensure!(
            result.is_err(),
            "expected an error for an input tensor of {len} bytes"
        );
        println!("[nn] rejected an input tensor of {len} bytes");
    }
    Ok(())
}
