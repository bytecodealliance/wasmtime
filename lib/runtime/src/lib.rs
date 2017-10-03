//! Standalone runtime for WebAssembly using Cretonne. Provides functions to translate
//! `get_global`, `set_global`, `current_memory`, `grow_memory`, `call_indirect` that hardcode in
//! the translation the base addresses of regions of memory that will hold the globals, tables and
//! linear memories.

#![deny(missing_docs)]

extern crate cretonne;
extern crate cton_wasm;

use cton_wasm::{FunctionIndex, GlobalIndex, TableIndex, MemoryIndex, Global, GlobalInit, Table,
                Memory, WasmRuntime, FuncEnvironment, GlobalValue, SignatureIndex};
use cretonne::ir::{InstBuilder, FuncRef, ExtFuncData, FunctionName, Signature, ArgumentType,
                   CallConv};
use cretonne::ir::types::*;
use cretonne::cursor::FuncCursor;
use cretonne::packed_option::PackedOption;
use cretonne::ir;
use cretonne::settings;
use cretonne::entity::EntityMap;
use std::mem::transmute;
use std::ptr::copy_nonoverlapping;
use std::ptr::write;

/// Runtime state of a WebAssembly table element.
#[derive(Clone, Debug)]
pub enum TableElement {
    /// A element that, if called, produces a trap.
    Trap(),
    /// A function.
    Function(FunctionIndex),
}

/// Information about a WebAssembly global variable.
pub struct GlobalInfo {
    global: Global,
    offset: usize,
}

/// Runtime state of a WebAssembly global variable.
pub struct GlobalsData {
    data: Vec<u8>,
    info: Vec<GlobalInfo>,
}

/// A description of a WebAssembly table.
pub struct TableData {
    /// The data stored in the table.
    pub data: Vec<u8>,
    /// Function indices to be stored in the table.
    pub elements: Vec<TableElement>,
    /// The description of the table.
    pub info: Table,
}

/// A description of a WebAssembly linear memory.
pub struct MemoryData {
    /// The data stored in the memory.
    pub data: Vec<u8>,
    /// The description of the memory.
    pub info: Memory,
}

const PAGE_SIZE: usize = 65536;

/// Object containing the standalone runtime information. To be passed after creation as argument
/// to `cton_wasm::translatemodule`.
pub struct Runtime {
    /// Compilation setting flags.
    flags: settings::Flags,

    /// Unprocessed signatures exactly as provided by `declare_signature()`.
    signatures: Vec<ir::Signature>,
    /// Types of functions, imported and local.
    func_types: Vec<SignatureIndex>,
    /// Names of imported functions.
    pub imported_funcs: Vec<ir::FunctionName>,

    /// WebAssembly global variables.
    pub globals: GlobalsData,
    /// WebAssembly tables.
    pub tables: Vec<TableData>,
    /// WebAssembly linear memories.
    pub memories: Vec<MemoryData>,
    instantiated: bool,

    has_current_memory: Option<FuncRef>,
    has_grow_memory: Option<FuncRef>,

    /// Mapping from cretonne FuncRef to wasm FunctionIndex.
    pub func_indices: EntityMap<FuncRef, FunctionIndex>,

    the_heap: PackedOption<ir::Heap>,

    current_global_offset: usize,
}

impl Runtime {
    /// Allocates the runtime data structures with default flags.
    pub fn default() -> Self {
        Self::with_flags(settings::Flags::new(&settings::builder()))
    }

    /// Allocates the runtime data structures with the given flags.
    pub fn with_flags(flags: settings::Flags) -> Self {
        Self {
            flags,
            signatures: Vec::new(),
            func_types: Vec::new(),
            imported_funcs: Vec::new(),
            globals: GlobalsData {
                data: Vec::new(),
                info: Vec::new(),
            },
            tables: Vec::new(),
            memories: Vec::new(),
            instantiated: false,
            has_current_memory: None,
            has_grow_memory: None,
            func_indices: EntityMap::new(),
            the_heap: PackedOption::default(),
            current_global_offset: 0,
        }
    }
}

impl FuncEnvironment for Runtime {
    fn flags(&self) -> &settings::Flags {
        &self.flags
    }

    fn make_global(&mut self, func: &mut ir::Function, index: GlobalIndex) -> GlobalValue {
        // Just create a dummy `vmctx` global.
        let offset = ((index * 8) as i32 + 8).into();
        let gv = func.create_global_var(ir::GlobalVarData::VmCtx { offset });
        GlobalValue::Memory {
            gv,
            ty: self.globals.info[index].global.ty,
        }
    }

    fn make_heap(&mut self, func: &mut ir::Function, _index: MemoryIndex) -> ir::Heap {
        debug_assert!(self.the_heap.is_none(), "multiple heaps not supported yet");

        let heap = func.create_heap(ir::HeapData {
            base: ir::HeapBase::ReservedReg,
            min_size: 0.into(),
            guard_size: 0x8000_0000.into(),
            style: ir::HeapStyle::Static { bound: 0x1_0000_0000.into() },
        });

        self.the_heap = PackedOption::from(heap);

        heap
    }

    fn make_indirect_sig(&mut self, func: &mut ir::Function, index: SignatureIndex) -> ir::SigRef {
        // A real implementation would probably change the calling convention and add `vmctx` and
        // signature index arguments.
        func.import_signature(self.signatures[index].clone())
    }

    fn make_direct_func(&mut self, func: &mut ir::Function, index: FunctionIndex) -> ir::FuncRef {
        let sigidx = self.func_types[index];
        // A real implementation would probably add a `vmctx` argument.
        // And maybe attempt some signature de-duplication.
        let signature = func.import_signature(self.signatures[sigidx].clone());

        let name = match self.imported_funcs.get(index) {
            Some(name) => name.clone(),
            None => ir::FunctionName::new(format!("localfunc{}", index)),
        };

        let func_ref = func.import_function(ir::ExtFuncData { name, signature });

        self.func_indices[func_ref] = index;

        func_ref
    }

    fn translate_call_indirect(
        &mut self,
        mut pos: FuncCursor,
        table_index: TableIndex,
        _sig_index: SignatureIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> ir::Inst {
        debug_assert_eq!(table_index, 0, "non-default tables not supported yet");
        pos.ins().call_indirect(sig_ref, callee, call_args)
    }

    fn translate_grow_memory(
        &mut self,
        mut pos: FuncCursor,
        index: MemoryIndex,
        heap: ir::Heap,
        val: ir::Value,
    ) -> ir::Value {
        debug_assert!(self.instantiated);
        debug_assert_eq!(index, 0, "non-default memories not supported yet");
        debug_assert_eq!(
            heap,
            self.the_heap.unwrap(),
            "multiple heaps not supported yet"
        );
        let grow_mem_func = self.has_grow_memory.unwrap_or_else(|| {
            let sig_ref = pos.func.import_signature(Signature {
                call_conv: CallConv::Native,
                argument_bytes: None,
                argument_types: vec![ArgumentType::new(I32)],
                return_types: vec![ArgumentType::new(I32)],
            });
            pos.func.import_function(ExtFuncData {
                name: FunctionName::new("grow_memory"),
                signature: sig_ref,
            })
        });
        self.has_grow_memory = Some(grow_mem_func);
        let call_inst = pos.ins().call(grow_mem_func, &[val]);
        *pos.func.dfg.inst_results(call_inst).first().unwrap()
    }

    fn translate_current_memory(
        &mut self,
        mut pos: FuncCursor,
        index: MemoryIndex,
        heap: ir::Heap,
    ) -> ir::Value {
        debug_assert!(self.instantiated);
        debug_assert_eq!(index, 0, "non-default memories not supported yet");
        debug_assert_eq!(
            heap,
            self.the_heap.unwrap(),
            "multiple heaps not supported yet"
        );
        let cur_mem_func = self.has_current_memory.unwrap_or_else(|| {
            let sig_ref = pos.func.import_signature(Signature {
                call_conv: CallConv::Native,
                argument_bytes: None,
                argument_types: Vec::new(),
                return_types: vec![ArgumentType::new(I32)],
            });
            pos.func.import_function(ExtFuncData {
                name: FunctionName::new("current_memory"),
                signature: sig_ref,
            })
        });
        self.has_current_memory = Some(cur_mem_func);
        let call_inst = pos.ins().call(cur_mem_func, &[]);
        *pos.func.dfg.inst_results(call_inst).first().unwrap()
    }
}

/// This trait is useful for
/// `cton_wasm::translatemodule` because it
/// tells how to translate runtime-dependent wasm instructions. These functions should not be
/// called by the user.
impl WasmRuntime for Runtime {
    fn declare_signature(&mut self, sig: &ir::Signature) {
        self.signatures.push(sig.clone());
    }

    fn declare_func_import(&mut self, sig_index: SignatureIndex, module: &[u8], field: &[u8]) {
        debug_assert_eq!(
            self.func_types.len(),
            self.imported_funcs.len(),
            "Imported functions must be declared first"
        );
        self.func_types.push(sig_index);

        // TODO: name_fold and concatenation with '_' are lossy; figure out something better.
        let mut name = Vec::new();
        name.extend(module.iter().cloned().map(name_fold));
        name.push(b'_');
        name.extend(field.iter().cloned().map(name_fold));
        self.imported_funcs.push(ir::FunctionName::new(name));
    }

    fn declare_func_type(&mut self, sig_index: SignatureIndex) {
        self.func_types.push(sig_index);
    }

    fn begin_translation(&mut self) {
        debug_assert!(!self.instantiated);
        self.instantiated = true;
        // At instantiation, we allocate memory for the globals, the memories and the tables
        // First the globals
        let mut globals_data_size = 0;
        for globalinfo in &mut self.globals.info {
            globalinfo.offset = globals_data_size;
            globals_data_size += globalinfo.global.ty.bytes() as usize;
        }
        self.globals.data.resize(globals_data_size, 0);
        for globalinfo in &self.globals.info {
            match globalinfo.global.initializer {
                GlobalInit::I32Const(val) => unsafe {
                    write(
                        self.globals.data.as_mut_ptr().offset(
                            globalinfo.offset as isize,
                        ) as *mut i32,
                        val,
                    )
                },
                GlobalInit::I64Const(val) => unsafe {
                    write(
                        self.globals.data.as_mut_ptr().offset(
                            globalinfo.offset as isize,
                        ) as *mut i64,
                        val,
                    )
                },
                GlobalInit::F32Const(val) => unsafe {
                    write(
                        self.globals.data.as_mut_ptr().offset(
                            globalinfo.offset as isize,
                        ) as *mut f32,
                        transmute(val),
                    )
                },
                GlobalInit::F64Const(val) => unsafe {
                    write(
                        self.globals.data.as_mut_ptr().offset(
                            globalinfo.offset as isize,
                        ) as *mut f64,
                        transmute(val),
                    )
                },
                GlobalInit::Import() => {
                    // We don't initialize, this is inter-module linking
                    // TODO: support inter-module imports
                }
                GlobalInit::GlobalRef(index) => {
                    let ref_offset = self.globals.info[index].offset;
                    let size = globalinfo.global.ty.bytes();
                    unsafe {
                        let dst = self.globals.data.as_mut_ptr().offset(
                            globalinfo.offset as isize,
                        );
                        let src = self.globals.data.as_ptr().offset(ref_offset as isize);
                        copy_nonoverlapping(src, dst, size as usize)
                    }
                }
            }
        }
    }
    fn next_function(&mut self) {
        self.has_current_memory = None;
        self.has_grow_memory = None;
        self.func_indices.clear();
        self.the_heap = PackedOption::default();
    }
    fn declare_global(&mut self, global: Global) {
        debug_assert!(!self.instantiated);
        self.globals.info.push(GlobalInfo {
            global: global,
            offset: self.current_global_offset,
        });
        self.current_global_offset += global.ty.bytes() as usize;
    }
    fn declare_table(&mut self, table: Table) {
        debug_assert!(!self.instantiated);
        let mut elements_vec = Vec::with_capacity(table.size);
        elements_vec.resize(table.size, TableElement::Trap());
        let mut addresses_vec = Vec::with_capacity(table.size);
        addresses_vec.resize(table.size, 0);
        self.tables.push(TableData {
            info: table,
            data: addresses_vec,
            elements: elements_vec,
        });
    }
    fn declare_table_elements(
        &mut self,
        table_index: TableIndex,
        offset: usize,
        elements: &[FunctionIndex],
    ) {
        debug_assert!(!self.instantiated);
        for (i, elt) in elements.iter().enumerate() {
            self.tables[table_index].elements[offset + i] = TableElement::Function(*elt);
        }
    }
    fn declare_memory(&mut self, memory: Memory) {
        debug_assert!(!self.instantiated);
        let mut memory_vec = Vec::with_capacity(memory.pages_count * PAGE_SIZE);
        memory_vec.resize(memory.pages_count * PAGE_SIZE, 0);
        self.memories.push(MemoryData {
            info: memory,
            data: memory_vec,
        });
    }
    fn declare_data_initialization(
        &mut self,
        memory_index: MemoryIndex,
        offset: usize,
        data: &[u8],
    ) -> Result<(), String> {
        if offset + data.len() > self.memories[memory_index].info.pages_count * PAGE_SIZE {
            return Err(String::from("initialization data out of bounds"));
        }
        self.memories[memory_index].data[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }
}

/// Convenience functions for the user to be called after execution for debug purposes.
impl Runtime {
    /// Returns a slice of the contents of allocated linear memory.
    pub fn inspect_memory(&self, memory_index: usize, address: usize, len: usize) -> &[u8] {
        &self.memories
            .get(memory_index)
            .expect(format!("no memory for index {}", memory_index).as_str())
            .data
            [address..address + len]
    }
    /// Shows the value of a global variable.
    pub fn inspect_global(&self, global_index: usize) -> &[u8] {
        let (offset, len) = (
            self.globals.info[global_index].offset,
            self.globals.info[global_index].global.ty.bytes() as usize,
        );
        &self.globals.data[offset..offset + len]
    }
}

// Generate characters suitable for printable `FuncName`s.
fn name_fold(c: u8) -> u8 {
    if (c as char).is_alphanumeric() {
        c
    } else {
        b'_'
    }
}
