use crate::read_debuginfo::WasmFileInfo;
use crate::transform::ModuleAddressMap;
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_wasm::DefinedFuncIndex;
use gimli::write;
use std::collections::BTreeMap;
use std::ops::Bound::{Included, Unbounded};
use std::vec::Vec;

pub type GeneratedAddress = usize;
pub type WasmAddress = u64;
pub type SymbolIndex = usize;

#[derive(Debug)]
pub struct AddressMap {
    pub generated: GeneratedAddress,
    pub wasm: WasmAddress,
}

#[derive(Debug)]
pub struct FunctionMap {
    pub offset: GeneratedAddress,
    pub len: GeneratedAddress,
    pub addresses: Box<[AddressMap]>,
}

#[derive(Debug)]
pub struct AddressTransform {
    lookup: BTreeMap<WasmAddress, (SymbolIndex, GeneratedAddress, GeneratedAddress)>,
    map: PrimaryMap<DefinedFuncIndex, FunctionMap>,
    func_ranges: Vec<(usize, usize)>,
}

impl AddressTransform {
    pub fn new(at: &ModuleAddressMap, wasm_file: &WasmFileInfo) -> Self {
        let code_section_offset = wasm_file.code_section_offset;
        let function_offsets = &wasm_file.function_offsets_and_sizes;
        let mut lookup = BTreeMap::new();
        let mut map = PrimaryMap::new();
        let mut func_ranges = Vec::new();
        for (i, ft) in at {
            let index = i.index();
            let (fn_offset, fn_size) = function_offsets[index];
            assert!(code_section_offset <= fn_offset);
            let fn_offset: WasmAddress = fn_offset - code_section_offset;
            let fn_size = fn_size as WasmAddress;
            func_ranges.push((ft.body_offset, ft.body_offset + ft.body_len));
            lookup.insert(
                fn_offset as WasmAddress,
                (index, ft.body_offset, ft.body_offset),
            );
            let mut fn_map = Vec::new();
            for t in &ft.instructions {
                if t.srcloc.is_default() {
                    // TODO extend some range if possible
                    continue;
                }
                // src_offset is a wasm bytecode offset in the code section
                let src_offset = t.srcloc.bits() as WasmAddress - code_section_offset;
                assert!(fn_offset <= src_offset && src_offset <= fn_offset + fn_size);
                lookup.insert(
                    src_offset,
                    (index, t.code_offset, t.code_offset + t.code_len),
                );
                fn_map.push(AddressMap {
                    generated: t.code_offset,
                    wasm: src_offset,
                });
            }
            let last_addr = ft.body_offset + ft.body_len;
            lookup.insert(fn_offset + fn_size, (index, last_addr, last_addr));
            fn_map.sort_by(|a, b| a.generated.cmp(&b.generated));
            map.push(FunctionMap {
                offset: ft.body_offset,
                len: ft.body_len,
                addresses: fn_map.into_boxed_slice(),
            });
        }
        AddressTransform {
            lookup,
            map,
            func_ranges,
        }
    }

    pub fn can_translate_address(&self, addr: u64) -> bool {
        self.translate(addr).is_some()
    }

    pub fn translate(&self, addr: u64) -> Option<write::Address> {
        if addr == 0 {
            // It's normally 0 for debug info without the linked code.
            return None;
        }
        let search = self.lookup.range((Unbounded, Included(addr)));
        if let Some((_, value)) = search.last() {
            return Some(write::Address::Symbol {
                symbol: value.0,
                addend: value.1 as i64,
            });
        }
        // Address was not found: function was not compiled?
        None
    }

    pub fn diff(&self, addr1: u64, addr2: u64) -> Option<u64> {
        let t1 = self.translate(addr1);
        let t2 = self.translate(addr2);
        if t1.is_none() || t2.is_none() {
            return None;
        }
        if let (
            Some(write::Address::Symbol {
                symbol: s1,
                addend: a,
            }),
            Some(write::Address::Symbol {
                symbol: s2,
                addend: b,
            }),
        ) = (t1, t2)
        {
            if s1 != s2 {
                panic!("different symbol");
            }
            Some((b - a) as u64)
        } else {
            unreachable!();
        }
    }

    pub fn delta(&self, addr1: u64, u: u64) -> Option<u64> {
        self.diff(addr1, addr1 + u)
    }

    pub fn map(&self) -> &PrimaryMap<DefinedFuncIndex, FunctionMap> {
        &self.map
    }

    pub fn func_range(&self, index: usize) -> (usize, usize) {
        self.func_ranges[index]
    }
}
