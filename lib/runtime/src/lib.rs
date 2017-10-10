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
                   CallConv, ArgumentPurpose, ArgumentLoc, ArgumentExtension, Function};
use cretonne::ir::types::*;
use cretonne::ir::immediates::Offset32;
use cretonne::cursor::FuncCursor;
use cretonne::packed_option::PackedOption;
use cretonne::ir;
use cretonne::settings;
use cretonne::entity::EntityMap;
use std::mem::transmute;
use std::ptr::copy_nonoverlapping;
use std::ptr::write;
use std::collections::HashMap;

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

/// An entity to export.
pub enum Export {
    /// Function export.
    Function(FunctionIndex),
    /// Table export.
    Table(TableIndex),
    /// Memory export.
    Memory(MemoryIndex),
    /// Global export.
    Global(GlobalIndex),
}

/// Object containing the standalone runtime information. To be passed after creation as argument
/// to `cton_wasm::translatemodule`.
pub struct Runtime {
    /// Compilation setting flags.
    flags: settings::Flags,

    /// Unprocessed signatures exactly as provided by `declare_signature()`.
    signatures: Vec<ir::Signature>,

    /// Names of imported functions.
    pub imported_funcs: Vec<(String, String)>,

    /// Types of functions, imported and local.
    functions: Vec<SignatureIndex>,

    /// WebAssembly tables.
    pub tables: Vec<TableData>,

    /// WebAssembly linear memories.
    pub memories: Vec<MemoryData>,

    /// WebAssembly global variables.
    pub globals: GlobalsData,

    /// Exported entities.
    pub exports: HashMap<String, Export>,

    instantiated: bool,

    has_current_memory: Option<FuncRef>,
    has_grow_memory: Option<FuncRef>,

    /// Mapping from cretonne FuncRef to wasm FunctionIndex.
    pub func_indices: EntityMap<FuncRef, FunctionIndex>,

    the_heap: PackedOption<ir::Heap>,

    /// The module "start" function, if present.
    pub start_func: Option<FunctionIndex>,
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
            imported_funcs: Vec::new(),
            functions: Vec::new(),
            tables: Vec::new(),
            memories: Vec::new(),
            globals: GlobalsData {
                data: Vec::new(),
                info: Vec::new(),
            },
            exports: HashMap::new(),
            instantiated: false,
            has_current_memory: None,
            has_grow_memory: None,
            func_indices: EntityMap::new(),
            the_heap: PackedOption::default(),
            start_func: None,
        }
    }

    /// Return the offset from the VmCtx pointer where global `index` is allocated.
    fn global_offset(index: GlobalIndex) -> usize {
        // Add one for the hidden heap base global.
        (index as usize + 1) * 8
    }

    /// Return the size of the VmCtx area needed to hold all currently declared globals.
    fn globals_data_size(&self) -> usize {
        // Add one for the hidden heap base global.
        (self.globals.info.len() + 1) * 8
    }

    /// Transform the call argument list in preparation for making a call.
    fn get_real_call_args(func: &Function, call_args: &[ir::Value]) -> Vec<ir::Value> {
        let mut real_call_args = Vec::with_capacity(call_args.len() + 1);
        real_call_args.extend_from_slice(call_args);
        real_call_args.push(func.special_arg(ArgumentPurpose::VMContext).unwrap());
        real_call_args
    }
}

impl FuncEnvironment for Runtime {
    fn flags(&self) -> &settings::Flags {
        &self.flags
    }

    fn make_global(&mut self, func: &mut ir::Function, index: GlobalIndex) -> GlobalValue {
        let offset = Self::global_offset(index);
        let offset32 = offset as i32;
        debug_assert_eq!(offset32 as usize, offset);
        let gv =
            func.create_global_var(ir::GlobalVarData::VmCtx { offset: Offset32::new(offset32) });
        GlobalValue::Memory {
            gv,
            ty: self.globals.info[index].global.ty,
        }
    }

    fn make_heap(&mut self, func: &mut ir::Function, _index: MemoryIndex) -> ir::Heap {
        debug_assert!(self.the_heap.is_none(), "multiple heaps not supported yet");

        let heap_base =
            func.create_global_var(ir::GlobalVarData::VmCtx { offset: Offset32::new(0) });

        let heap = func.create_heap(ir::HeapData {
            base: ir::HeapBase::GlobalVar(heap_base),
            min_size: 0.into(),
            guard_size: 0x8000_0000.into(),
            style: ir::HeapStyle::Static { bound: 0x1_0000_0000.into() },
        });

        self.the_heap = PackedOption::from(heap);

        heap
    }

    fn make_indirect_sig(&mut self, func: &mut ir::Function, index: SignatureIndex) -> ir::SigRef {
        func.import_signature(self.signatures[index].clone())
    }

    fn make_direct_func(&mut self, func: &mut ir::Function, index: FunctionIndex) -> ir::FuncRef {
        let sigidx = self.functions[index];
        let signature = func.import_signature(self.signatures[sigidx].clone());
        let name = self.get_func_name(index);
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
        let real_call_args = Self::get_real_call_args(pos.func, call_args);
        pos.ins().call_indirect(sig_ref, callee, &real_call_args)
    }

    fn translate_call(
        &mut self,
        mut pos: FuncCursor,
        _callee_index: FunctionIndex,
        callee: ir::FuncRef,
        call_args: &[ir::Value],
    ) -> ir::Inst {
        let real_call_args = Self::get_real_call_args(pos.func, call_args);
        pos.ins().call(callee, &real_call_args)
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
    fn get_func_name(&self, func_index: FunctionIndex) -> cretonne::ir::FunctionName {
        ir::FunctionName::new(format!("wasm_0x{:x}", func_index))
    }

    fn declare_signature(&mut self, sig: &ir::Signature) {
        let mut sig = sig.clone();
        sig.argument_types.push(ArgumentType {
            value_type: self.native_pointer(),
            purpose: ArgumentPurpose::VMContext,
            extension: ArgumentExtension::None,
            location: ArgumentLoc::Unassigned,
        });
        // TODO: Deduplicate signatures.
        self.signatures.push(sig);
    }

    fn get_signature(&self, sig_index: SignatureIndex) -> &ir::Signature {
        &self.signatures[sig_index]
    }

    fn declare_func_import(&mut self, sig_index: SignatureIndex, module: &str, field: &str) {
        debug_assert_eq!(
            self.functions.len(),
            self.imported_funcs.len(),
            "Imported functions must be declared first"
        );
        self.functions.push(sig_index);

        self.imported_funcs.push((
            String::from(module),
            String::from(field),
        ));
    }

    fn get_num_func_imports(&self) -> usize {
        self.imported_funcs.len()
    }

    fn declare_func_type(&mut self, sig_index: SignatureIndex) {
        self.functions.push(sig_index);
    }

    fn get_func_type(&self, func_index: FunctionIndex) -> usize {
        self.functions[func_index]
    }

    fn declare_global(&mut self, global: Global) {
        debug_assert!(!self.instantiated);
        let index = self.globals.info.len() as GlobalIndex;
        self.globals.info.push(GlobalInfo {
            global: global,
            offset: Self::global_offset(index),
        });
    }

    fn get_global(&self, global_index: GlobalIndex) -> &cton_wasm::Global {
        &self.globals.info[global_index].global
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
        base: Option<GlobalIndex>,
        offset: usize,
        elements: &[FunctionIndex],
    ) {
        debug_assert!(base.is_none(), "global-value offsets not supported yet");
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
        base: Option<GlobalIndex>,
        offset: usize,
        data: &[u8],
    ) {
        debug_assert!(base.is_none(), "global-value offsets not supported yet");
        debug_assert!(
            offset + data.len() <= self.memories[memory_index].info.pages_count * PAGE_SIZE,
            "initialization data out of bounds"
        );
        self.memories[memory_index].data[offset..offset + data.len()].copy_from_slice(data);
    }

    fn declare_func_export(&mut self, func_index: FunctionIndex, name: &str) {
        self.exports.insert(
            String::from(name),
            Export::Function(func_index),
        );
    }

    fn declare_table_export(&mut self, table_index: TableIndex, name: &str) {
        self.exports.insert(
            String::from(name),
            Export::Table(table_index),
        );
    }

    fn declare_memory_export(&mut self, memory_index: MemoryIndex, name: &str) {
        self.exports.insert(
            String::from(name),
            Export::Memory(memory_index),
        );
    }

    fn declare_global_export(&mut self, global_index: GlobalIndex, name: &str) {
        self.exports.insert(
            String::from(name),
            Export::Global(global_index),
        );
    }

    fn declare_start_func(&mut self, func_index: FunctionIndex) {
        debug_assert!(self.start_func.is_none());
        self.start_func = Some(func_index);
    }

    fn begin_translation(&mut self) {
        debug_assert!(!self.instantiated);
        self.instantiated = true;
        // At instantiation, we allocate memory for the globals, the memories and the tables
        // First the globals
        let globals_data_size = self.globals_data_size();
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
