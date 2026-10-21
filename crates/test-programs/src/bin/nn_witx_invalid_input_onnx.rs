//! Send an input tensor with the wrong byte length to the ONNX backend.
//! Check that the backend returns an error and does not trap.

use anyhow::{Context, Result, ensure};
use std::fs;
use test_programs::nn::witx::{self, TensorType};

/// The byte length of the `[1, 3, 224, 224]` `f32` input of MobileNet.
const INPUT_BYTES: usize = 1 * 3 * 224 * 224 * 4;

pub fn main() -> Result<()> {
    let model = fs::read("fixture/model.onnx")
        .context("the model file to be mapped to the fixture directory")?;
    let graph = witx::load(
        &[&model],
        witx::GraphEncoding::Onnx,
        witx::ExecutionTarget::CPU,
    )?;
    let mut context = graph.init_execution_context()?;

    for len in [INPUT_BYTES - 1, INPUT_BYTES + 1] {
        let result = context.set_input(0, TensorType::F32, &[1, 3, 224, 224], &vec![0u8; len]);
        ensure!(
            result.is_err(),
            "expected an error for an input tensor of {len} bytes"
        );
        println!("[nn] rejected an input tensor of {len} bytes");
    }
    Ok(())
}
