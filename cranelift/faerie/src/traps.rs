//! Faerie trap manifests record every `TrapCode` that cranelift outputs during code generation,
//! for every function in the module. This data may be useful at runtime.

use cranelift_codegen::{binemit, ir};
use cranelift_module::TrapSite;

/// Record of the trap sites for a given function
#[derive(Debug)]
pub struct FaerieTrapSink {
    /// Name of function
    pub name: String,
    /// Total code size of function
    pub code_size: u32,
    /// All trap sites collected in function
    pub sites: Vec<TrapSite>,
}

impl FaerieTrapSink {
    /// Create an empty `FaerieTrapSink`
    pub fn new(name: &str, code_size: u32) -> Self {
        Self {
            sites: Vec::new(),
            name: name.to_owned(),
            code_size,
        }
    }

    /// Create a `FaerieTrapSink` pre-populated with `traps`
    pub fn new_with_sites(name: &str, code_size: u32, traps: Vec<TrapSite>) -> Self {
        Self {
            sites: traps,
            name: name.to_owned(),
            code_size,
        }
    }
}

impl binemit::TrapSink for FaerieTrapSink {
    fn trap(&mut self, offset: binemit::CodeOffset, srcloc: ir::SourceLoc, code: ir::TrapCode) {
        self.sites.push(TrapSite {
            offset,
            srcloc,
            code,
        });
    }
}

/// Collection of all `FaerieTrapSink`s for the module
#[derive(Debug)]
pub struct FaerieTrapManifest {
    /// All `FaerieTrapSink` for the module
    pub sinks: Vec<FaerieTrapSink>,
}

impl FaerieTrapManifest {
    /// Create an empty `FaerieTrapManifest`
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    /// Put a `FaerieTrapSink` into manifest
    pub fn add_sink(&mut self, sink: FaerieTrapSink) {
        self.sinks.push(sink);
    }
}
