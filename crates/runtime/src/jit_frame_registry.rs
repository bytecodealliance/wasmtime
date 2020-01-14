#![allow(missing_docs)]

use lazy_static::lazy_static;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

lazy_static! {
    static ref REGISTRY: RwLock<JITFrameRegistry> = RwLock::new(JITFrameRegistry::default());
}

#[derive(Clone)]
pub struct JITFrameTag {
    pub module_id: Option<String>,
    pub func_index: usize,
}

impl std::fmt::Debug for JITFrameTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref module_id) = self.module_id {
            write!(f, "{}", module_id)?;
        } else {
            write!(f, "(module)")?;
        }
        write!(f, ":{}", self.func_index)
    }
}

struct JITFrameRegistry {
    ranges: BTreeMap<usize, (usize, Arc<JITFrameTag>)>,
}

impl Default for JITFrameRegistry {
    fn default() -> Self {
        Self {
            ranges: Default::default(),
        }
    }
}

impl JITFrameRegistry {
    fn register(&mut self, fn_start: usize, fn_end: usize, tag: JITFrameTag) {
        self.ranges.insert(fn_end, (fn_start, Arc::new(tag)));
    }

    fn unregister(&mut self, fn_end: usize) {
        self.ranges.remove(&fn_end);
    }

    fn find(&self, pc: usize) -> Option<&Arc<JITFrameTag>> {
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

pub fn register(fn_start: usize, fn_end: usize, tag: JITFrameTag) {
    REGISTRY
        .write()
        .expect("jit frame registry lock got poisoned")
        .register(fn_start, fn_end, tag);
}

pub fn unregister(_fn_start: usize, fn_end: usize) {
    REGISTRY
        .write()
        .expect("jit frame registry lock got poisoned")
        .unregister(fn_end);
}

pub fn find(pc: usize) -> Option<Arc<JITFrameTag>> {
    REGISTRY
        .read()
        .expect("jit frame registry lock got poisoned")
        .find(pc)
        .cloned()
}
