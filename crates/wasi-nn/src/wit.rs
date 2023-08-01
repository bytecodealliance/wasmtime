//! Contains the macro-generated implementation of wasi-nn from the its witx definition file.
use crate::ctx::WasiNnCtx;
use crate::ctx::WasiNnError;
use anyhow::Result;
use wasmtime::component::bindgen;



bindgen!(in "spec/wit");


