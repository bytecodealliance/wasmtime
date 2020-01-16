#![allow(missing_docs)]

use lazy_static::lazy_static;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use wasmtime_environ::ModuleSyncString;

lazy_static! {
    static ref REGISTRY: RwLock<JITFunctionRegistry> = RwLock::new(JITFunctionRegistry::default());
}

#[derive(Clone)]
pub struct JITFunctionTag {
    pub module_id: ModuleSyncString,
    pub func_index: usize,
    pub func_name: ModuleSyncString,
}

impl std::fmt::Debug for JITFunctionTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref module_id) = self.module_id.get() {
            write!(f, "{}", module_id)?;
        } else {
            write!(f, "(module)")?;
        }
        write!(f, ":{}", self.func_index)
    }
}

struct JITFunctionRegistry {
    ranges: BTreeMap<usize, (usize, Arc<JITFunctionTag>)>,
}

impl Default for JITFunctionRegistry {
    fn default() -> Self {
        Self {
            ranges: Default::default(),
        }
    }
}

impl JITFunctionRegistry {
    fn register(&mut self, fn_start: usize, fn_end: usize, tag: JITFunctionTag) {
        self.ranges.insert(fn_end, (fn_start, Arc::new(tag)));
    }

    fn unregister(&mut self, fn_end: usize) {
        self.ranges.remove(&fn_end);
    }

    fn find(&self, pc: usize) -> Option<&Arc<JITFunctionTag>> {
        self.ranges
            .range(pc..)
            .next()
            .and_then(|(end, (start, s))| {
                if *start <= pc && pc < *end {
                    Some(s)
                } else {
                    None
                }
            })
    }
}

pub fn register(fn_start: usize, fn_end: usize, tag: JITFunctionTag) {
    REGISTRY
        .write()
        .expect("jit function registry lock got poisoned")
        .register(fn_start, fn_end, tag);
}

pub fn unregister(_fn_start: usize, fn_end: usize) {
    REGISTRY
        .write()
        .expect("jit function registry lock got poisoned")
        .unregister(fn_end);
}

pub fn find(pc: usize) -> Option<Arc<JITFunctionTag>> {
    REGISTRY
        .read()
        .expect("jit function registry lock got poisoned")
        .find(pc)
        .cloned()
}
