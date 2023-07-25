//! Configuration of `.wat` tests.
//!
//! The config is the leading `;;!` comments in the WAT. It is in TOML.

use anyhow::{bail, ensure, Result};
use cranelift_codegen::ir;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestConfig {
    #[serde(default)]
    pub target: String,

    #[serde(default)]
    pub compile: bool,

    #[serde(default)]
    pub optimize: bool,

    #[serde(default)]
    pub settings: Vec<String>,

    #[serde(default)]
    pub globals: BTreeMap<String, TestGlobal>,

    #[serde(default)]
    pub heaps: Vec<TestHeap>,

    #[serde(default)]
    pub relaxed_simd_deterministic: bool,
}

impl TestConfig {
    pub fn validate(&self) -> Result<()> {
        if self.compile || self.optimize {
            ensure!(
                !(self.compile && self.optimize),
                "The `compile` and `optimize` options are mutually exclusive."
            );
        }

        for global in self.globals.values() {
            ensure!(
                global.vmctx || global.load.is_some(),
                "global must be either `vmctx` or a `load`"
            );
            ensure!(
                !(global.vmctx && global.load.is_some()),
                "global cannot be both a `vmctx` and a `load`"
            );

            if let Some(load) = &global.load {
                ensure!(
                    self.globals.contains_key(&load.base),
                    "global's load base must be another global"
                );
            }
        }

        for heap in &self.heaps {
            ensure!(
                self.globals.contains_key(&heap.base),
                "heap base must be a declared global"
            );

            match heap.style.kind.as_str() {
                "static" => match &heap.style.bound {
                    toml::value::Value::Integer(x) => {
                        ensure!(*x >= 0, "static heap bound cannot be negative")
                    }
                    _ => bail!("static heap bounds must be integers"),
                },
                "dynamic" => match &heap.style.bound {
                    toml::value::Value::String(g) => {
                        ensure!(
                            self.globals.contains_key(g),
                            "dynamic heap bound must be a declared global"
                        )
                    }
                    _ => bail!("dynamic heap bounds must be strings"),
                },
                other => {
                    bail!(
                        "heap style must be 'static' or 'dynamic', found '{}'",
                        other
                    )
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestGlobal {
    #[serde(rename = "type")]
    pub type_: String,

    #[serde(default)]
    pub vmctx: bool,

    #[serde(default)]
    pub load: Option<TestGlobalLoad>,
}

impl TestGlobal {
    pub fn to_ir(
        &self,
        name_to_ir_global: &BTreeMap<String, ir::GlobalValue>,
    ) -> ir::GlobalValueData {
        if self.vmctx {
            ir::GlobalValueData::VMContext
        } else if let Some(load) = &self.load {
            ir::GlobalValueData::Load {
                base: name_to_ir_global[&load.base],
                offset: i32::try_from(load.offset).unwrap().into(),
                global_type: match self.type_.as_str() {
                    "i32" => ir::types::I32,
                    "i64" => ir::types::I64,
                    other => panic!("test globals cannot be of type '{other}'"),
                },
                readonly: load.readonly,
            }
        } else {
            unreachable!()
        }
    }

    pub fn dependencies<'a>(&'a self) -> impl Iterator<Item = &'a str> + 'a {
        let mut deps = None;
        if let Some(load) = &self.load {
            deps = Some(load.base.as_str());
        }
        deps.into_iter()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestGlobalLoad {
    pub base: String,

    #[serde(default)]
    pub offset: u32,

    #[serde(default)]
    pub readonly: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestHeap {
    pub base: String,

    #[serde(default)]
    pub min_size: u64,

    #[serde(default)]
    pub offset_guard_size: u64,

    pub style: TestHeapStyle,

    pub index_type: String,
}

impl TestHeap {
    pub fn to_ir(
        &self,
        name_to_ir_global: &BTreeMap<String, ir::GlobalValue>,
    ) -> cranelift_wasm::HeapData {
        cranelift_wasm::HeapData {
            base: name_to_ir_global[&self.base],
            min_size: self.min_size.into(),
            offset_guard_size: self.offset_guard_size.into(),
            style: self.style.to_ir(name_to_ir_global),
            index_type: match self.index_type.as_str() {
                "i32" => ir::types::I32,
                "i64" => ir::types::I64,
                other => panic!("heap indices may only be i32 or i64, found '{other}'"),
            },
        }
    }

    pub fn dependencies<'a>(&'a self) -> impl Iterator<Item = &'a str> + 'a {
        let mut deps = vec![self.base.as_str()];
        if self.style.kind == "dynamic" {
            deps.push(match &self.style.bound {
                toml::Value::String(g) => g.as_str(),
                _ => unreachable!(),
            });
        }
        deps.into_iter()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestHeapStyle {
    pub kind: String,
    pub bound: toml::value::Value,
}

impl TestHeapStyle {
    pub fn to_ir(
        &self,
        name_to_ir_global: &BTreeMap<String, ir::GlobalValue>,
    ) -> cranelift_wasm::HeapStyle {
        match self.kind.as_str() {
            "static" => cranelift_wasm::HeapStyle::Static {
                bound: match &self.bound {
                    toml::Value::Integer(x) => u64::try_from(*x).unwrap().into(),
                    _ => unreachable!(),
                },
            },
            "dynamic" => cranelift_wasm::HeapStyle::Dynamic {
                bound_gv: match &self.bound {
                    toml::Value::String(g) => name_to_ir_global[g],
                    _ => unreachable!(),
                },
            },
            _ => unreachable!(),
        }
    }
}
