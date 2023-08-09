//! The `wasi-nn` types used internally in this crate.
//!
//! These types form a common "ground truth" for the `witx` and `wit` ABI types
//! to be converted from and to. As such, these types should be kept up to date
//! with the WIT and WITX specifications; if anything changes in the
//! specifications, we should see compile errors in the conversion functions
//! (e.g., `impl From<witx::...> for `crate::...`).

pub struct Tensor<'a> {
    pub dims: &'a [usize],
    pub ty: TensorType,
    pub data: &'a [u8],
}

#[derive(Clone, Copy)]
pub enum TensorType {
    F16,
    F32,
    U8,
    I32,
}

pub enum ExecutionTarget {
    CPU,
    GPU,
    TPU,
}

#[derive(Debug)]
pub enum GraphEncoding {
    OpenVINO,
    ONNX,
    Tensorflow,
    PyTorch,
    TensorflowLite,
}
