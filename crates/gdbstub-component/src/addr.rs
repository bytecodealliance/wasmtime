//! Synthetic Wasm address space expected by the gdbstub Wasm
//! extensions.

use crate::api::{Debuggee, Frame, Memory, Module};
use anyhow::Result;
use gdbstub_arch::wasm::addr::{WasmAddr, WasmAddrType};
use std::collections::{HashMap, hash_map::Entry};

/// Representation of the synthesized Wasm address space.
pub struct AddrSpace {
    module_ids: HashMap<u64, u32>,
    memory_ids: HashMap<u64, u32>,
    modules: Vec<Module>,
    module_bytecode: Vec<Vec<u8>>,
    memories: Vec<Memory>,
}

/// The result of a lookup in the address space.
pub enum AddrSpaceLookup<'a> {
    Module {
        module: &'a Module,
        bytecode: &'a [u8],
        offset: u32,
    },
    Memory {
        memory: &'a Memory,
        offset: u32,
    },
    Empty,
}

impl AddrSpace {
    pub fn new() -> Self {
        AddrSpace {
            module_ids: HashMap::new(),
            modules: vec![],
            module_bytecode: vec![],
            memory_ids: HashMap::new(),
            memories: vec![],
        }
    }

    fn module_id(&mut self, m: &Module) -> u32 {
        match self.module_ids.entry(m.unique_id()) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                let id = u32::try_from(self.modules.len()).unwrap();
                let bytecode = m.bytecode().unwrap_or(vec![]);
                self.module_bytecode.push(bytecode);
                self.modules.push(m.clone());
                *v.insert(id)
            }
        }
    }

    fn memory_id(&mut self, m: &Memory) -> u32 {
        match self.memory_ids.entry(m.unique_id()) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                let id = u32::try_from(self.memories.len()).unwrap();
                self.memories.push(m.clone());
                *v.insert(id)
            }
        }
    }

    /// Update/create new mappings so that all modules and instances'
    /// memories in the debuggee have mappings.
    pub fn update(&mut self, d: &Debuggee) -> Result<()> {
        for module in d.all_modules() {
            let _ = self.module_id(&module);
        }
        for instance in d.all_instances() {
            let mut idx = 0;
            loop {
                if let Ok(m) = instance.get_memory(d, idx) {
                    let _ = self.memory_id(&m);
                    idx += 1;
                } else {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Iterate over the base `WasmAddr` of every registered module.
    pub fn module_base_addrs(&self) -> impl Iterator<Item = WasmAddr> + '_ {
        (0..self.modules.len())
            .map(|idx| WasmAddr::new(WasmAddrType::Object, u32::try_from(idx).unwrap(), 0).unwrap())
    }

    /// Build the GDB memory-map XML describing all known regions.
    ///
    /// Module bytecode regions are reported as `rom` (read-only), and
    /// linear memories as `ram` (read-write).
    pub fn memory_map_xml(&self, debuggee: &Debuggee) -> String {
        use std::fmt::Write;
        let mut xml = String::from(
            "<?xml version=\"1.0\"?><!DOCTYPE memory-map SYSTEM \"memory-map.dtd\"><memory-map>",
        );
        for (idx, bc) in self.module_bytecode.iter().enumerate() {
            let start =
                WasmAddr::new(WasmAddrType::Object, u32::try_from(idx).unwrap(), 0).unwrap();
            let len = bc.len();
            if len > 0 {
                write!(
                    xml,
                    "<memory type=\"rom\" start=\"0x{:x}\" length=\"0x{:x}\"/>",
                    start.as_raw(),
                    len
                )
                .unwrap();
            }
        }
        for (idx, mem) in self.memories.iter().enumerate() {
            let start =
                WasmAddr::new(WasmAddrType::Memory, u32::try_from(idx).unwrap(), 0).unwrap();
            let len = mem.size_bytes(debuggee);
            if len > 0 {
                write!(
                    xml,
                    "<memory type=\"ram\" start=\"0x{:x}\" length=\"0x{:x}\"/>",
                    start.as_raw(),
                    len
                )
                .unwrap();
            }
        }
        xml.push_str("</memory-map>");
        xml
    }

    pub fn frame_to_pc(&self, frame: &Frame, debuggee: &Debuggee) -> WasmAddr {
        let module = frame.get_instance(debuggee).unwrap().get_module(debuggee);
        let &module_id = self
            .module_ids
            .get(&module.unique_id())
            .expect("module not found in addr space");
        let pc = frame.get_pc(debuggee).unwrap();
        WasmAddr::new(WasmAddrType::Object, module_id, pc).unwrap()
    }

    pub fn frame_to_return_addr(&self, frame: &Frame, debuggee: &Debuggee) -> Option<WasmAddr> {
        let module = frame.get_instance(debuggee).unwrap().get_module(debuggee);
        let &module_id = self
            .module_ids
            .get(&module.unique_id())
            .expect("module not found in addr space");
        let ret_pc = frame.get_pc(debuggee).ok()?;
        Some(WasmAddr::new(WasmAddrType::Object, module_id, ret_pc).unwrap())
    }

    pub fn lookup(&self, addr: WasmAddr, d: &Debuggee) -> AddrSpaceLookup<'_> {
        let index = usize::try_from(addr.module_index()).unwrap();
        match addr.addr_type() {
            WasmAddrType::Object => {
                if index >= self.modules.len() {
                    return AddrSpaceLookup::Empty;
                }
                let bytecode = &self.module_bytecode[index];
                if addr.offset() >= u32::try_from(bytecode.len()).unwrap() {
                    return AddrSpaceLookup::Empty;
                }
                AddrSpaceLookup::Module {
                    module: &self.modules[index],
                    bytecode,
                    offset: addr.offset(),
                }
            }
            WasmAddrType::Memory => {
                if index >= self.memories.len() {
                    return AddrSpaceLookup::Empty;
                }
                let size = self.memories[index].size_bytes(d);
                if u64::from(addr.offset()) >= size {
                    return AddrSpaceLookup::Empty;
                }
                AddrSpaceLookup::Memory {
                    memory: &self.memories[index],
                    offset: addr.offset(),
                }
            }
        }
    }
}
