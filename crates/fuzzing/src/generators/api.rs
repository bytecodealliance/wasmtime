//! Generating sequences of Wasmtime API calls.
//!
//! We only generate *valid* sequences of API calls. To do this, we keep track
//! of what objects we've already created in earlier API calls via the `Scope`
//! struct.
//!
//! To generate even-more-pathological sequences of API calls, we use [swarm
//! testing]:
//!
//! > In swarm testing, the usual practice of potentially including all features
//! > in every test case is abandoned. Rather, a large “swarm” of randomly
//! > generated configurations, each of which omits some features, is used, with
//! > configurations receiving equal resources.
//!
//! [swarm testing]: https://www.cs.utah.edu/~regehr/papers/swarm12.pdf

use crate::generators::Config;
use arbitrary::{Arbitrary, Unstructured};
use std::collections::{BTreeMap, BTreeSet};
use wasmtime::ValType;

/// The subset of Wasm value types that can be used for host-created globals.
#[derive(Arbitrary, Debug, Clone, Copy)]
#[expect(missing_docs, reason = "self-describing variants")]
pub enum FuzzValType {
    I32,
    I64,
    F32,
    F64,
}

impl FuzzValType {
    /// Convert to the corresponding `wasmtime::ValType`.
    pub fn to_wasmtime(self) -> ValType {
        match self {
            Self::I32 => ValType::I32,
            Self::I64 => ValType::I64,
            Self::F32 => ValType::F32,
            Self::F64 => ValType::F64,
        }
    }
}

#[derive(Arbitrary, Debug)]
struct Swarm {
    store_new: bool,
    store_drop: bool,
    store_gc: bool,
    module_new: bool,
    module_drop: bool,
    instance_new: bool,
    instance_drop: bool,
    call_exported_func: bool,
    global_type_new: bool,
    global_type_drop: bool,
    global_new: bool,
    global_get: bool,
    global_set: bool,
    global_ty: bool,
    global_drop: bool,
    get_global_export: bool,
    table_type_new: bool,
    table_type_drop: bool,
    table_new: bool,
    table_get: bool,
    table_set: bool,
    table_grow: bool,
    table_size: bool,
    table_ty: bool,
    table_drop: bool,
    get_table_export: bool,
    memory_type_new: bool,
    memory_type_drop: bool,
    memory_new: bool,
    memory_read: bool,
    memory_write: bool,
    memory_data: bool,
    memory_data_mut: bool,
    memory_grow: bool,
    memory_data_size: bool,
    memory_size: bool,
    memory_page_size: bool,
    memory_page_size_log2: bool,
    memory_ty: bool,
    memory_drop: bool,
    get_memory_export: bool,
    func_type_new: bool,
    func_type_drop: bool,
    func_new: bool,
    func_ty: bool,
    func_call: bool,
    get_func_export: bool,
    func_drop: bool,
    tag_type_new: bool,
    tag_type_drop: bool,
    tag_new: bool,
    tag_ty: bool,
    tag_eq: bool,
    get_tag_export: bool,
    tag_drop: bool,
    module_imports: bool,
    module_exports: bool,
    module_get_export: bool,
    module_name: bool,
    module_validate: bool,
    module_serialize_deserialize: bool,
    table_copy: bool,
    table_fill: bool,
    struct_type_new: bool,
    struct_type_drop: bool,
    struct_ref_pre_new: bool,
    struct_ref_pre_drop: bool,
    struct_ref_new: bool,
    struct_ref_ty: bool,
    struct_ref_field: bool,
    struct_ref_set_field: bool,
    struct_ref_drop: bool,
    array_type_new: bool,
    array_type_drop: bool,
    array_ref_pre_new: bool,
    array_ref_pre_drop: bool,
    array_ref_new: bool,
    array_ref_new_fixed: bool,
    array_ref_ty: bool,
    array_ref_len: bool,
    array_ref_get: bool,
    array_ref_set: bool,
    array_ref_drop: bool,
    exn_type_new: bool,
    exn_type_from_tag_type: bool,
    exn_type_drop: bool,
    exn_ref_pre_new: bool,
    exn_ref_pre_drop: bool,
    exn_ref_new: bool,
    exn_ref_ty: bool,
    exn_ref_tag: bool,
    exn_ref_field: bool,
    exn_ref_drop: bool,
}

/// A call to one of Wasmtime's public APIs.
#[derive(Arbitrary, Debug)]
#[expect(missing_docs, reason = "self-describing fields")]
pub enum ApiCall {
    StoreNew {
        id: usize,
        config: Config,
    },
    StoreDrop {
        id: usize,
    },
    StoreGc {
        id: usize,
    },
    ModuleNew {
        id: usize,
        wasm: Vec<u8>,
    },
    ModuleDrop {
        id: usize,
    },
    InstanceNew {
        id: usize,
        module: usize,
        store: usize,
    },
    InstanceDrop {
        id: usize,
    },
    CallExportedFunc {
        instance: usize,
        nth: usize,
    },
    GlobalTypeNew {
        id: usize,
        ty: FuzzValType,
        mutable: bool,
    },
    GlobalTypeDrop {
        id: usize,
    },
    GlobalNew {
        id: usize,
        global_ty: usize,
        store: usize,
    },
    GlobalGet {
        global: usize,
    },
    GlobalSet {
        global: usize,
    },
    GlobalTy {
        global: usize,
    },
    GlobalDrop {
        id: usize,
    },
    GetGlobalExport {
        id: usize,
        instance: usize,
        nth: usize,
    },
    TableTypeNew {
        id: usize,
        nullable: bool,
    },
    TableTypeDrop {
        id: usize,
    },
    TableNew {
        id: usize,
        table_ty: usize,
        store: usize,
    },
    TableGet {
        table: usize,
        idx: u64,
    },
    TableSet {
        table: usize,
        idx: u64,
    },
    TableGrow {
        table: usize,
        delta: u32,
    },
    TableSize {
        table: usize,
    },
    TableTy {
        table: usize,
    },
    TableDrop {
        id: usize,
    },
    GetTableExport {
        id: usize,
        instance: usize,
        nth: usize,
    },
    MemoryTypeNew {
        id: usize,
        minimum: u32,
        maximum: Option<u32>,
    },
    MemoryTypeDrop {
        id: usize,
    },
    MemoryNew {
        id: usize,
        memory_ty: usize,
        store: usize,
    },
    MemoryRead {
        memory: usize,
        offset: usize,
        len: usize,
    },
    MemoryWrite {
        memory: usize,
        offset: usize,
        data: Vec<u8>,
    },
    MemoryData {
        memory: usize,
    },
    MemoryDataMut {
        memory: usize,
    },
    MemoryGrow {
        memory: usize,
        delta: u32,
    },
    MemoryDataSize {
        memory: usize,
    },
    MemorySize {
        memory: usize,
    },
    MemoryPageSize {
        memory: usize,
    },
    MemoryPageSizeLog2 {
        memory: usize,
    },
    MemoryTy {
        memory: usize,
    },
    MemoryDrop {
        id: usize,
    },
    GetMemoryExport {
        id: usize,
        instance: usize,
        nth: usize,
    },
    FuncTypeNew {
        id: usize,
        params: Vec<FuzzValType>,
        results: Vec<FuzzValType>,
    },
    FuncTypeDrop {
        id: usize,
    },
    FuncNew {
        id: usize,
        func_ty: usize,
        store: usize,
    },
    FuncTy {
        func: usize,
    },
    FuncCall {
        func: usize,
    },
    GetFuncExport {
        id: usize,
        instance: usize,
        nth: usize,
    },
    FuncDrop {
        id: usize,
    },
    TagTypeNew {
        id: usize,
        func_ty: usize,
    },
    TagTypeDrop {
        id: usize,
    },
    TagNew {
        id: usize,
        tag_ty: usize,
        store: usize,
    },
    TagTy {
        tag: usize,
    },
    TagEq {
        a: usize,
        b: usize,
    },
    GetTagExport {
        id: usize,
        instance: usize,
        nth: usize,
    },
    TagDrop {
        id: usize,
    },
    ModuleImports {
        module: usize,
    },
    ModuleExports {
        module: usize,
    },
    ModuleGetExport {
        module: usize,
        name: String,
    },
    ModuleName {
        module: usize,
    },
    ModuleValidate {
        wasm: Vec<u8>,
    },
    ModuleSerializeDeserialize {
        src_id: usize,
        dst_id: usize,
    },
    TableCopy {
        dst_table: usize,
        dst_index: u64,
        src_table: usize,
        src_index: u64,
        len: u64,
    },
    TableFill {
        table: usize,
        dst: u64,
        len: u64,
    },
    StructTypeNew {
        id: usize,
        fields: Vec<(FuzzValType, bool)>,
    },
    StructTypeDrop {
        id: usize,
    },
    StructRefPreNew {
        id: usize,
        struct_ty: usize,
        store: usize,
    },
    StructRefPreDrop {
        id: usize,
    },
    StructRefNew {
        id: usize,
        pre: usize,
    },
    StructRefTy {
        struct_ref: usize,
    },
    StructRefField {
        struct_ref: usize,
        index: usize,
    },
    StructRefSetField {
        struct_ref: usize,
        index: usize,
    },
    StructRefDrop {
        id: usize,
    },
    ArrayTypeNew {
        id: usize,
        elem_ty: FuzzValType,
        mutable: bool,
    },
    ArrayTypeDrop {
        id: usize,
    },
    ArrayRefPreNew {
        id: usize,
        array_ty: usize,
        store: usize,
    },
    ArrayRefPreDrop {
        id: usize,
    },
    ArrayRefNew {
        id: usize,
        pre: usize,
        len: u32,
    },
    ArrayRefNewFixed {
        id: usize,
        pre: usize,
        count: u8,
    },
    ArrayRefTy {
        array_ref: usize,
    },
    ArrayRefLen {
        array_ref: usize,
    },
    ArrayRefGet {
        array_ref: usize,
        index: u32,
    },
    ArrayRefSet {
        array_ref: usize,
        index: u32,
    },
    ArrayRefDrop {
        id: usize,
    },
    ExnTypeNew {
        id: usize,
        fields: Vec<FuzzValType>,
    },
    ExnTypeFromTagType {
        id: usize,
        tag_ty: usize,
    },
    ExnTypeDrop {
        id: usize,
    },
    ExnRefPreNew {
        id: usize,
        exn_ty: usize,
        store: usize,
    },
    ExnRefPreDrop {
        id: usize,
    },
    ExnRefNew {
        id: usize,
        pre: usize,
        tag: usize,
    },
    ExnRefTy {
        exn_ref: usize,
    },
    ExnRefTag {
        exn_ref: usize,
    },
    ExnRefField {
        exn_ref: usize,
        index: usize,
    },
    ExnRefDrop {
        id: usize,
    },
}
use ApiCall::*;

struct Scope {
    id_counter: usize,

    /// Stores that are currently live.
    stores: BTreeSet<usize>,

    /// Modules that are currently live.
    modules: BTreeSet<usize>,

    /// Instances that are currently live. Maps from `instance_id` to the
    /// instance's associated `store_id`.
    instances: BTreeMap<usize, usize>,

    /// Global types that are currently live.
    global_types: BTreeSet<usize>,

    /// Globals that are currently live. Maps from `global_id` to the global's
    /// associated `store_id`.
    globals: BTreeMap<usize, usize>, // global_id -> store_id

    /// Table types that are currently live.
    table_types: BTreeSet<usize>,

    /// Tables that are currently live. Maps from `table_id` to the table's
    /// associated `store_id`.
    tables: BTreeMap<usize, usize>, // table_id -> store_id

    /// Memory types that are currently live.
    memory_types: BTreeSet<usize>,

    /// Memories that are currently live. Maps from `memory_id` to the memory's
    /// associated `store_id`.
    memories: BTreeMap<usize, usize>, // memory_id -> store_id

    /// Func types that are currently live.
    func_types: BTreeSet<usize>,

    /// Funcs that are currently live. Maps from `func_id` to the func's
    /// associated `store_id`.
    funcs: BTreeMap<usize, usize>, // func_id -> store_id

    /// Tag types that are currently live.
    tag_types: BTreeSet<usize>,

    /// Tags that are currently live. Maps from `tag_id` to the tag's
    /// associated `store_id`.
    tags: BTreeMap<usize, usize>, // tag_id -> store_id

    /// Struct types that are currently live.
    struct_types: BTreeSet<usize>,

    /// StructRefPres that are currently live. Maps from `pre_id` to the store's id.
    struct_ref_pres: BTreeMap<usize, usize>, // pre_id -> store_id

    /// StructRefs that are currently live. Maps from `ref_id` to the store's id.
    struct_refs: BTreeMap<usize, usize>, // ref_id -> store_id

    /// Array types that are currently live.
    array_types: BTreeSet<usize>,

    /// ArrayRefPres that are currently live. Maps from `pre_id` to the store's id.
    array_ref_pres: BTreeMap<usize, usize>, // pre_id -> store_id

    /// ArrayRefs that are currently live. Maps from `ref_id` to the store's id.
    array_refs: BTreeMap<usize, usize>, // ref_id -> store_id

    /// Exception types that are currently live.
    exn_types: BTreeSet<usize>,

    /// ExnRefPres that are currently live. Maps from `pre_id` to the store's id.
    exn_ref_pres: BTreeMap<usize, usize>, // pre_id -> store_id

    /// ExnRefs that are currently live. Maps from `ref_id` to the store's id.
    exn_refs: BTreeMap<usize, usize>, // ref_id -> store_id

    config: Config,
}

impl Scope {
    fn next_id(&mut self) -> usize {
        let id = self.id_counter;
        self.id_counter = id + 1;
        id
    }
}

/// A sequence of API calls.
#[derive(Debug)]
pub struct ApiCalls {
    /// The API calls.
    pub calls: Vec<ApiCall>,
}

impl<'a> Arbitrary<'a> for ApiCalls {
    fn arbitrary(input: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        crate::init_fuzzing();

        let swarm = Swarm::arbitrary(input)?;
        let mut calls = vec![];

        let config = Config::arbitrary(input)?;
        let mut scope = Scope {
            id_counter: 0,
            stores: BTreeSet::default(),
            modules: BTreeSet::default(),
            instances: BTreeMap::default(),
            global_types: BTreeSet::default(),
            globals: BTreeMap::default(),
            table_types: BTreeSet::default(),
            tables: BTreeMap::default(),
            memory_types: BTreeSet::default(),
            memories: BTreeMap::default(),
            func_types: BTreeSet::default(),
            funcs: BTreeMap::default(),
            tag_types: BTreeSet::default(),
            tags: BTreeMap::default(),
            struct_types: BTreeSet::default(),
            struct_ref_pres: BTreeMap::default(),
            struct_refs: BTreeMap::default(),
            array_types: BTreeSet::default(),
            array_ref_pres: BTreeMap::default(),
            array_refs: BTreeMap::default(),
            exn_types: BTreeSet::default(),
            exn_ref_pres: BTreeMap::default(),
            exn_refs: BTreeMap::default(),
            config: config.clone(),
        };

        let store_id = scope.next_id();
        scope.stores.insert(store_id);
        calls.push(StoreNew {
            id: store_id,
            config,
        });

        // Total limit on number of API calls we'll generate. This exists to
        // avoid libFuzzer timeouts.
        let max_calls = 100;

        let mut choices: Vec<fn(&mut Unstructured<'a>, &mut Scope) -> arbitrary::Result<ApiCall>> =
            vec![];

        for _ in 0..std::cmp::min(max_calls, input.arbitrary_len::<ApiCall>()?) {
            choices.clear();

            if swarm.store_new {
                choices.push(|_input, scope| {
                    let id = scope.next_id();
                    scope.stores.insert(id);
                    Ok(StoreNew {
                        id,
                        config: scope.config.clone(),
                    })
                });
            }
            if swarm.store_drop && scope.stores.len() > 1 {
                choices.push(|input, scope| {
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let id = **input.choose(&stores)?;
                    scope.stores.remove(&id);
                    scope.instances.retain(|_, store_id| *store_id != id);
                    scope.globals.retain(|_, store_id| *store_id != id);
                    scope.tables.retain(|_, store_id| *store_id != id);
                    scope.memories.retain(|_, store_id| *store_id != id);
                    scope.funcs.retain(|_, store_id| *store_id != id);
                    scope.tags.retain(|_, store_id| *store_id != id);
                    scope.struct_ref_pres.retain(|_, store_id| *store_id != id);
                    scope.struct_refs.retain(|_, store_id| *store_id != id);
                    scope.array_ref_pres.retain(|_, store_id| *store_id != id);
                    scope.array_refs.retain(|_, store_id| *store_id != id);
                    scope.exn_ref_pres.retain(|_, store_id| *store_id != id);
                    scope.exn_refs.retain(|_, store_id| *store_id != id);
                    Ok(StoreDrop { id })
                });
            }
            if swarm.store_gc && !scope.stores.is_empty() {
                choices.push(|input, scope| {
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let id = **input.choose(&stores)?;
                    Ok(StoreGc { id })
                });
            }
            if swarm.module_new {
                choices.push(|input, scope| {
                    let id = scope.next_id();
                    let wasm = scope.config.generate(input, Some(1000))?;
                    scope.modules.insert(id);
                    Ok(ModuleNew {
                        id,
                        wasm: wasm.to_bytes(),
                    })
                });
            }
            if swarm.module_drop && !scope.modules.is_empty() {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.iter().collect();
                    let id = **input.choose(&modules)?;
                    scope.modules.remove(&id);
                    Ok(ModuleDrop { id })
                });
            }
            if swarm.instance_new && !scope.modules.is_empty() && !scope.stores.is_empty() {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.iter().collect();
                    let module = **input.choose(&modules)?;
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let store = **input.choose(&stores)?;
                    let id = scope.next_id();
                    scope.instances.insert(id, store);
                    Ok(InstanceNew { id, module, store })
                });
            }
            if swarm.instance_drop && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.keys().collect();
                    let id = **input.choose(&instances)?;
                    scope.instances.remove(&id);
                    Ok(InstanceDrop { id })
                });
            }
            if swarm.call_exported_func && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.keys().collect();
                    let instance = **input.choose(&instances)?;
                    let nth = usize::arbitrary(input)?;
                    Ok(CallExportedFunc { instance, nth })
                });
            }
            if swarm.global_type_new {
                choices.push(|input, scope| {
                    let id = scope.next_id();
                    let ty = FuzzValType::arbitrary(input)?;
                    let mutable = bool::arbitrary(input)?;
                    scope.global_types.insert(id);
                    Ok(GlobalTypeNew { id, ty, mutable })
                });
            }
            if swarm.global_type_drop && !scope.global_types.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.global_types.iter().collect();
                    let id = **input.choose(&types)?;
                    scope.global_types.remove(&id);
                    Ok(GlobalTypeDrop { id })
                });
            }
            if swarm.global_new && !scope.global_types.is_empty() && !scope.stores.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.global_types.iter().collect();
                    let global_ty = **input.choose(&types)?;
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let store = **input.choose(&stores)?;
                    let id = scope.next_id();
                    scope.globals.insert(id, store);
                    Ok(GlobalNew {
                        id,
                        global_ty,
                        store,
                    })
                });
            }
            if swarm.global_get && !scope.globals.is_empty() {
                choices.push(|input, scope| {
                    let globals: Vec<_> = scope.globals.keys().collect();
                    let global = **input.choose(&globals)?;
                    Ok(GlobalGet { global })
                });
            }
            if swarm.global_set && !scope.globals.is_empty() {
                choices.push(|input, scope| {
                    let globals: Vec<_> = scope.globals.keys().collect();
                    let global = **input.choose(&globals)?;
                    Ok(GlobalSet { global })
                });
            }
            if swarm.global_ty && !scope.globals.is_empty() {
                choices.push(|input, scope| {
                    let globals: Vec<_> = scope.globals.keys().collect();
                    let global = **input.choose(&globals)?;
                    Ok(GlobalTy { global })
                });
            }
            if swarm.global_drop && !scope.globals.is_empty() {
                choices.push(|input, scope| {
                    let globals: Vec<_> = scope.globals.keys().collect();
                    let id = **input.choose(&globals)?;
                    scope.globals.remove(&id);
                    Ok(GlobalDrop { id })
                });
            }
            if swarm.get_global_export && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.keys().collect();
                    let instance = **input.choose(&instances)?;
                    let nth = usize::arbitrary(input)?;
                    let id = scope.next_id();
                    let store = *scope.instances.get(&instance).unwrap();
                    scope.globals.insert(id, store);
                    Ok(GetGlobalExport { id, instance, nth })
                });
            }
            if swarm.table_type_new {
                choices.push(|input, scope| {
                    let id = scope.next_id();
                    let nullable = bool::arbitrary(input)?;
                    scope.table_types.insert(id);
                    Ok(TableTypeNew { id, nullable })
                });
            }
            if swarm.table_type_drop && !scope.table_types.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.table_types.iter().collect();
                    let id = **input.choose(&types)?;
                    scope.table_types.remove(&id);
                    Ok(TableTypeDrop { id })
                });
            }
            if swarm.table_new && !scope.table_types.is_empty() && !scope.stores.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.table_types.iter().collect();
                    let table_ty = **input.choose(&types)?;
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let store = **input.choose(&stores)?;
                    let id = scope.next_id();
                    scope.tables.insert(id, store);
                    Ok(TableNew {
                        id,
                        table_ty,
                        store,
                    })
                });
            }
            if swarm.table_get && !scope.tables.is_empty() {
                choices.push(|input, scope| {
                    let tables: Vec<_> = scope.tables.keys().collect();
                    let table = **input.choose(&tables)?;
                    let idx = u64::arbitrary(input)?;
                    Ok(TableGet { table, idx })
                });
            }
            if swarm.table_set && !scope.tables.is_empty() {
                choices.push(|input, scope| {
                    let tables: Vec<_> = scope.tables.keys().collect();
                    let table = **input.choose(&tables)?;
                    let idx = u64::arbitrary(input)?;
                    Ok(TableSet { table, idx })
                });
            }
            if swarm.table_grow && !scope.tables.is_empty() {
                choices.push(|input, scope| {
                    let tables: Vec<_> = scope.tables.keys().collect();
                    let table = **input.choose(&tables)?;
                    let delta = input.int_in_range(0..=100)?;
                    Ok(TableGrow { table, delta })
                });
            }
            if swarm.table_size && !scope.tables.is_empty() {
                choices.push(|input, scope| {
                    let tables: Vec<_> = scope.tables.keys().collect();
                    let table = **input.choose(&tables)?;
                    Ok(TableSize { table })
                });
            }
            if swarm.table_ty && !scope.tables.is_empty() {
                choices.push(|input, scope| {
                    let tables: Vec<_> = scope.tables.keys().collect();
                    let table = **input.choose(&tables)?;
                    Ok(TableTy { table })
                });
            }
            if swarm.table_drop && !scope.tables.is_empty() {
                choices.push(|input, scope| {
                    let tables: Vec<_> = scope.tables.keys().collect();
                    let id = **input.choose(&tables)?;
                    scope.tables.remove(&id);
                    Ok(TableDrop { id })
                });
            }
            if swarm.get_table_export && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.keys().collect();
                    let instance = **input.choose(&instances)?;
                    let nth = usize::arbitrary(input)?;
                    let id = scope.next_id();
                    let store = *scope.instances.get(&instance).unwrap();
                    scope.tables.insert(id, store);
                    Ok(GetTableExport { id, instance, nth })
                });
            }
            if swarm.memory_type_new {
                choices.push(|input, scope| {
                    let id = scope.next_id();
                    let minimum = u32::arbitrary(input)? % 10;
                    let has_max = bool::arbitrary(input)?;
                    let maximum = if has_max {
                        Some(minimum + u32::arbitrary(input)? % 10)
                    } else {
                        None
                    };
                    scope.memory_types.insert(id);
                    Ok(MemoryTypeNew {
                        id,
                        minimum,
                        maximum,
                    })
                });
            }
            if swarm.memory_type_drop && !scope.memory_types.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.memory_types.iter().collect();
                    let id = **input.choose(&types)?;
                    scope.memory_types.remove(&id);
                    Ok(MemoryTypeDrop { id })
                });
            }
            if swarm.memory_new && !scope.memory_types.is_empty() && !scope.stores.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.memory_types.iter().collect();
                    let memory_ty = **input.choose(&types)?;
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let store = **input.choose(&stores)?;
                    let id = scope.next_id();
                    scope.memories.insert(id, store);
                    Ok(MemoryNew {
                        id,
                        memory_ty,
                        store,
                    })
                });
            }
            if swarm.memory_read && !scope.memories.is_empty() {
                choices.push(|input, scope| {
                    let memories: Vec<_> = scope.memories.keys().collect();
                    let memory = **input.choose(&memories)?;
                    let offset = usize::arbitrary(input)?;
                    let len = usize::arbitrary(input)? % 64;
                    Ok(MemoryRead {
                        memory,
                        offset,
                        len,
                    })
                });
            }
            if swarm.memory_write && !scope.memories.is_empty() {
                choices.push(|input, scope| {
                    let memories: Vec<_> = scope.memories.keys().collect();
                    let memory = **input.choose(&memories)?;
                    let offset = usize::arbitrary(input)?;
                    let data = Vec::<u8>::arbitrary(input)?;
                    Ok(MemoryWrite {
                        memory,
                        offset,
                        data,
                    })
                });
            }
            if swarm.memory_data && !scope.memories.is_empty() {
                choices.push(|input, scope| {
                    let memories: Vec<_> = scope.memories.keys().collect();
                    let memory = **input.choose(&memories)?;
                    Ok(MemoryData { memory })
                });
            }
            if swarm.memory_data_mut && !scope.memories.is_empty() {
                choices.push(|input, scope| {
                    let memories: Vec<_> = scope.memories.keys().collect();
                    let memory = **input.choose(&memories)?;
                    Ok(MemoryDataMut { memory })
                });
            }
            if swarm.memory_grow && !scope.memories.is_empty() {
                choices.push(|input, scope| {
                    let memories: Vec<_> = scope.memories.keys().collect();
                    let memory = **input.choose(&memories)?;
                    let delta = input.int_in_range(0..=10)?;
                    Ok(MemoryGrow { memory, delta })
                });
            }
            if swarm.memory_data_size && !scope.memories.is_empty() {
                choices.push(|input, scope| {
                    let memories: Vec<_> = scope.memories.keys().collect();
                    let memory = **input.choose(&memories)?;
                    Ok(MemoryDataSize { memory })
                });
            }
            if swarm.memory_size && !scope.memories.is_empty() {
                choices.push(|input, scope| {
                    let memories: Vec<_> = scope.memories.keys().collect();
                    let memory = **input.choose(&memories)?;
                    Ok(MemorySize { memory })
                });
            }
            if swarm.memory_page_size && !scope.memories.is_empty() {
                choices.push(|input, scope| {
                    let memories: Vec<_> = scope.memories.keys().collect();
                    let memory = **input.choose(&memories)?;
                    Ok(MemoryPageSize { memory })
                });
            }
            if swarm.memory_page_size_log2 && !scope.memories.is_empty() {
                choices.push(|input, scope| {
                    let memories: Vec<_> = scope.memories.keys().collect();
                    let memory = **input.choose(&memories)?;
                    Ok(MemoryPageSizeLog2 { memory })
                });
            }
            if swarm.memory_ty && !scope.memories.is_empty() {
                choices.push(|input, scope| {
                    let memories: Vec<_> = scope.memories.keys().collect();
                    let memory = **input.choose(&memories)?;
                    Ok(MemoryTy { memory })
                });
            }
            if swarm.memory_drop && !scope.memories.is_empty() {
                choices.push(|input, scope| {
                    let memories: Vec<_> = scope.memories.keys().collect();
                    let id = **input.choose(&memories)?;
                    scope.memories.remove(&id);
                    Ok(MemoryDrop { id })
                });
            }
            if swarm.get_memory_export && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.keys().collect();
                    let instance = **input.choose(&instances)?;
                    let nth = usize::arbitrary(input)?;
                    let id = scope.next_id();
                    let store = *scope.instances.get(&instance).unwrap();
                    scope.memories.insert(id, store);
                    Ok(GetMemoryExport { id, instance, nth })
                });
            }
            if swarm.func_type_new {
                choices.push(|input, scope| {
                    let id = scope.next_id();
                    let params = Vec::<FuzzValType>::arbitrary(input)?;
                    let params: Vec<_> = params.into_iter().take(4).collect();
                    let results = Vec::<FuzzValType>::arbitrary(input)?;
                    let results: Vec<_> = results.into_iter().take(4).collect();
                    scope.func_types.insert(id);
                    Ok(FuncTypeNew {
                        id,
                        params,
                        results,
                    })
                });
            }
            if swarm.func_type_drop && !scope.func_types.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.func_types.iter().collect();
                    let id = **input.choose(&types)?;
                    scope.func_types.remove(&id);
                    Ok(FuncTypeDrop { id })
                });
            }
            if swarm.func_new && !scope.func_types.is_empty() && !scope.stores.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.func_types.iter().collect();
                    let func_ty = **input.choose(&types)?;
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let store = **input.choose(&stores)?;
                    let id = scope.next_id();
                    scope.funcs.insert(id, store);
                    Ok(FuncNew { id, func_ty, store })
                });
            }
            if swarm.func_ty && !scope.funcs.is_empty() {
                choices.push(|input, scope| {
                    let funcs: Vec<_> = scope.funcs.keys().collect();
                    let func = **input.choose(&funcs)?;
                    Ok(FuncTy { func })
                });
            }
            if swarm.func_call && !scope.funcs.is_empty() {
                choices.push(|input, scope| {
                    let funcs: Vec<_> = scope.funcs.keys().collect();
                    let func = **input.choose(&funcs)?;
                    Ok(FuncCall { func })
                });
            }
            if swarm.get_func_export && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.keys().collect();
                    let instance = **input.choose(&instances)?;
                    let nth = usize::arbitrary(input)?;
                    let id = scope.next_id();
                    let store = *scope.instances.get(&instance).unwrap();
                    scope.funcs.insert(id, store);
                    Ok(GetFuncExport { id, instance, nth })
                });
            }
            if swarm.func_drop && !scope.funcs.is_empty() {
                choices.push(|input, scope| {
                    let funcs: Vec<_> = scope.funcs.keys().collect();
                    let id = **input.choose(&funcs)?;
                    scope.funcs.remove(&id);
                    Ok(FuncDrop { id })
                });
            }
            if swarm.tag_type_new && !scope.func_types.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.func_types.iter().collect();
                    let func_ty = **input.choose(&types)?;
                    let id = scope.next_id();
                    scope.tag_types.insert(id);
                    Ok(TagTypeNew { id, func_ty })
                });
            }
            if swarm.tag_type_drop && !scope.tag_types.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.tag_types.iter().collect();
                    let id = **input.choose(&types)?;
                    scope.tag_types.remove(&id);
                    Ok(TagTypeDrop { id })
                });
            }
            if swarm.tag_new && !scope.tag_types.is_empty() && !scope.stores.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.tag_types.iter().collect();
                    let tag_ty = **input.choose(&types)?;
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let store = **input.choose(&stores)?;
                    let id = scope.next_id();
                    scope.tags.insert(id, store);
                    Ok(TagNew { id, tag_ty, store })
                });
            }
            if swarm.tag_ty && !scope.tags.is_empty() {
                choices.push(|input, scope| {
                    let tags: Vec<_> = scope.tags.keys().collect();
                    let tag = **input.choose(&tags)?;
                    Ok(TagTy { tag })
                });
            }
            if swarm.tag_eq && scope.tags.len() >= 2 {
                choices.push(|input, scope| {
                    let tags: Vec<_> = scope.tags.keys().collect();
                    let a = **input.choose(&tags)?;
                    let b = **input.choose(&tags)?;
                    Ok(TagEq { a, b })
                });
            }
            if swarm.get_tag_export && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.keys().collect();
                    let instance = **input.choose(&instances)?;
                    let nth = usize::arbitrary(input)?;
                    let id = scope.next_id();
                    let store = *scope.instances.get(&instance).unwrap();
                    scope.tags.insert(id, store);
                    Ok(GetTagExport { id, instance, nth })
                });
            }
            if swarm.tag_drop && !scope.tags.is_empty() {
                choices.push(|input, scope| {
                    let tags: Vec<_> = scope.tags.keys().collect();
                    let id = **input.choose(&tags)?;
                    scope.tags.remove(&id);
                    Ok(TagDrop { id })
                });
            }
            if swarm.module_imports && !scope.modules.is_empty() {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.iter().collect();
                    let module = **input.choose(&modules)?;
                    Ok(ModuleImports { module })
                });
            }
            if swarm.module_exports && !scope.modules.is_empty() {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.iter().collect();
                    let module = **input.choose(&modules)?;
                    Ok(ModuleExports { module })
                });
            }
            if swarm.module_get_export && !scope.modules.is_empty() {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.iter().collect();
                    let module = **input.choose(&modules)?;
                    let name = String::arbitrary(input)?;
                    Ok(ModuleGetExport { module, name })
                });
            }
            if swarm.module_name && !scope.modules.is_empty() {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.iter().collect();
                    let module = **input.choose(&modules)?;
                    Ok(ModuleName { module })
                });
            }
            if swarm.module_validate {
                choices.push(|input, scope| {
                    let use_valid_wasm = bool::arbitrary(input)?;
                    let wasm = if use_valid_wasm {
                        scope.config.generate(input, Some(1000))?.to_bytes()
                    } else {
                        Vec::<u8>::arbitrary(input)?
                    };
                    Ok(ModuleValidate { wasm })
                });
            }
            if swarm.module_serialize_deserialize && !scope.modules.is_empty() {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.iter().collect();
                    let src_id = **input.choose(&modules)?;
                    let dst_id = scope.next_id();
                    scope.modules.insert(dst_id);
                    Ok(ModuleSerializeDeserialize { src_id, dst_id })
                });
            }
            if swarm.table_copy && scope.tables.len() >= 2 {
                choices.push(|input, scope| {
                    // Find two table ids that map to the same store.
                    let by_store: std::collections::BTreeMap<usize, Vec<usize>> = scope
                        .tables
                        .iter()
                        .fold(Default::default(), |mut m, (&tid, &sid)| {
                            m.entry(sid).or_default().push(tid);
                            m
                        });
                    // Only proceed if at least one store has 2+ tables.
                    let valid_stores: Vec<_> =
                        by_store.iter().filter(|(_, ts)| ts.len() >= 2).collect();
                    if valid_stores.is_empty() {
                        // Fall back: pick any two tables (may be different stores; oracle skips).
                        let tables: Vec<_> = scope.tables.keys().collect();
                        let dst_table = **input.choose(&tables)?;
                        let src_table = **input.choose(&tables)?;
                        let dst_index = u64::arbitrary(input)?;
                        let src_index = u64::arbitrary(input)?;
                        let len = u64::arbitrary(input)? % 8;
                        return Ok(TableCopy {
                            dst_table,
                            dst_index,
                            src_table,
                            src_index,
                            len,
                        });
                    }
                    let (_, store_tables) = *input.choose(&valid_stores)?;
                    let dst_table = *input.choose(store_tables)?;
                    let src_table = *input.choose(store_tables)?;
                    let dst_index = u64::arbitrary(input)?;
                    let src_index = u64::arbitrary(input)?;
                    let len = u64::arbitrary(input)? % 8;
                    Ok(TableCopy {
                        dst_table,
                        dst_index,
                        src_table,
                        src_index,
                        len,
                    })
                });
            }
            if swarm.table_fill && !scope.tables.is_empty() {
                choices.push(|input, scope| {
                    let tables: Vec<_> = scope.tables.keys().collect();
                    let table = **input.choose(&tables)?;
                    let dst = u64::arbitrary(input)?;
                    let len = u64::arbitrary(input)? % 8;
                    Ok(TableFill { table, dst, len })
                });
            }
            if swarm.struct_type_new {
                choices.push(|input, scope| {
                    let id = scope.next_id();
                    let fields = Vec::<(FuzzValType, bool)>::arbitrary(input)?;
                    let fields: Vec<_> = fields.into_iter().take(4).collect();
                    scope.struct_types.insert(id);
                    Ok(StructTypeNew { id, fields })
                });
            }
            if swarm.struct_type_drop && !scope.struct_types.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.struct_types.iter().collect();
                    let id = **input.choose(&types)?;
                    scope.struct_types.remove(&id);
                    Ok(StructTypeDrop { id })
                });
            }
            if swarm.struct_ref_pre_new
                && !scope.struct_types.is_empty()
                && !scope.stores.is_empty()
            {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.struct_types.iter().collect();
                    let struct_ty = **input.choose(&types)?;
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let store = **input.choose(&stores)?;
                    let id = scope.next_id();
                    scope.struct_ref_pres.insert(id, store);
                    Ok(StructRefPreNew {
                        id,
                        struct_ty,
                        store,
                    })
                });
            }
            if swarm.struct_ref_pre_drop && !scope.struct_ref_pres.is_empty() {
                choices.push(|input, scope| {
                    let pres: Vec<_> = scope.struct_ref_pres.keys().collect();
                    let id = **input.choose(&pres)?;
                    scope.struct_ref_pres.remove(&id);
                    Ok(StructRefPreDrop { id })
                });
            }
            if swarm.struct_ref_new && !scope.struct_ref_pres.is_empty() {
                choices.push(|input, scope| {
                    let pres: Vec<_> = scope.struct_ref_pres.iter().collect();
                    let (&pre, &store_id) = *input.choose(&pres)?;
                    let id = scope.next_id();
                    scope.struct_refs.insert(id, store_id);
                    Ok(StructRefNew { id, pre })
                });
            }
            if swarm.struct_ref_ty && !scope.struct_refs.is_empty() {
                choices.push(|input, scope| {
                    let refs: Vec<_> = scope.struct_refs.keys().collect();
                    let struct_ref = **input.choose(&refs)?;
                    Ok(StructRefTy { struct_ref })
                });
            }
            if swarm.struct_ref_field && !scope.struct_refs.is_empty() {
                choices.push(|input, scope| {
                    let refs: Vec<_> = scope.struct_refs.keys().collect();
                    let struct_ref = **input.choose(&refs)?;
                    let index = usize::arbitrary(input)?;
                    Ok(StructRefField { struct_ref, index })
                });
            }
            if swarm.struct_ref_set_field && !scope.struct_refs.is_empty() {
                choices.push(|input, scope| {
                    let refs: Vec<_> = scope.struct_refs.keys().collect();
                    let struct_ref = **input.choose(&refs)?;
                    let index = usize::arbitrary(input)?;
                    Ok(StructRefSetField { struct_ref, index })
                });
            }
            if swarm.struct_ref_drop && !scope.struct_refs.is_empty() {
                choices.push(|input, scope| {
                    let refs: Vec<_> = scope.struct_refs.keys().collect();
                    let id = **input.choose(&refs)?;
                    scope.struct_refs.remove(&id);
                    Ok(StructRefDrop { id })
                });
            }
            if swarm.array_type_new {
                choices.push(|input, scope| {
                    let id = scope.next_id();
                    let elem_ty = FuzzValType::arbitrary(input)?;
                    let mutable = bool::arbitrary(input)?;
                    scope.array_types.insert(id);
                    Ok(ArrayTypeNew {
                        id,
                        elem_ty,
                        mutable,
                    })
                });
            }
            if swarm.array_type_drop && !scope.array_types.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.array_types.iter().collect();
                    let id = **input.choose(&types)?;
                    scope.array_types.remove(&id);
                    Ok(ArrayTypeDrop { id })
                });
            }
            if swarm.array_ref_pre_new && !scope.array_types.is_empty() && !scope.stores.is_empty()
            {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.array_types.iter().collect();
                    let array_ty = **input.choose(&types)?;
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let store = **input.choose(&stores)?;
                    let id = scope.next_id();
                    scope.array_ref_pres.insert(id, store);
                    Ok(ArrayRefPreNew {
                        id,
                        array_ty,
                        store,
                    })
                });
            }
            if swarm.array_ref_pre_drop && !scope.array_ref_pres.is_empty() {
                choices.push(|input, scope| {
                    let pres: Vec<_> = scope.array_ref_pres.keys().collect();
                    let id = **input.choose(&pres)?;
                    scope.array_ref_pres.remove(&id);
                    Ok(ArrayRefPreDrop { id })
                });
            }
            if swarm.array_ref_new && !scope.array_ref_pres.is_empty() {
                choices.push(|input, scope| {
                    let pres: Vec<_> = scope.array_ref_pres.iter().collect();
                    let (&pre, &store_id) = *input.choose(&pres)?;
                    let id = scope.next_id();
                    let len = u32::arbitrary(input)? % 17;
                    scope.array_refs.insert(id, store_id);
                    Ok(ArrayRefNew { id, pre, len })
                });
            }
            if swarm.array_ref_new_fixed && !scope.array_ref_pres.is_empty() {
                choices.push(|input, scope| {
                    let pres: Vec<_> = scope.array_ref_pres.iter().collect();
                    let (&pre, &store_id) = *input.choose(&pres)?;
                    let id = scope.next_id();
                    let count = u8::arbitrary(input)? % 9;
                    scope.array_refs.insert(id, store_id);
                    Ok(ArrayRefNewFixed { id, pre, count })
                });
            }
            if swarm.array_ref_ty && !scope.array_refs.is_empty() {
                choices.push(|input, scope| {
                    let refs: Vec<_> = scope.array_refs.keys().collect();
                    let array_ref = **input.choose(&refs)?;
                    Ok(ArrayRefTy { array_ref })
                });
            }
            if swarm.array_ref_len && !scope.array_refs.is_empty() {
                choices.push(|input, scope| {
                    let refs: Vec<_> = scope.array_refs.keys().collect();
                    let array_ref = **input.choose(&refs)?;
                    Ok(ArrayRefLen { array_ref })
                });
            }
            if swarm.array_ref_get && !scope.array_refs.is_empty() {
                choices.push(|input, scope| {
                    let refs: Vec<_> = scope.array_refs.keys().collect();
                    let array_ref = **input.choose(&refs)?;
                    let index = u32::arbitrary(input)?;
                    Ok(ArrayRefGet { array_ref, index })
                });
            }
            if swarm.array_ref_set && !scope.array_refs.is_empty() {
                choices.push(|input, scope| {
                    let refs: Vec<_> = scope.array_refs.keys().collect();
                    let array_ref = **input.choose(&refs)?;
                    let index = u32::arbitrary(input)?;
                    Ok(ArrayRefSet { array_ref, index })
                });
            }
            if swarm.array_ref_drop && !scope.array_refs.is_empty() {
                choices.push(|input, scope| {
                    let refs: Vec<_> = scope.array_refs.keys().collect();
                    let id = **input.choose(&refs)?;
                    scope.array_refs.remove(&id);
                    Ok(ArrayRefDrop { id })
                });
            }
            if swarm.exn_type_new {
                choices.push(|input, scope| {
                    let id = scope.next_id();
                    let fields = Vec::<FuzzValType>::arbitrary(input)?;
                    let fields: Vec<_> = fields.into_iter().take(4).collect();
                    scope.exn_types.insert(id);
                    Ok(ExnTypeNew { id, fields })
                });
            }
            if swarm.exn_type_from_tag_type && !scope.tag_types.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.tag_types.iter().collect();
                    let tag_ty = **input.choose(&types)?;
                    let id = scope.next_id();
                    scope.exn_types.insert(id);
                    Ok(ExnTypeFromTagType { id, tag_ty })
                });
            }
            if swarm.exn_type_drop && !scope.exn_types.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.exn_types.iter().collect();
                    let id = **input.choose(&types)?;
                    scope.exn_types.remove(&id);
                    Ok(ExnTypeDrop { id })
                });
            }
            if swarm.exn_ref_pre_new && !scope.exn_types.is_empty() && !scope.stores.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.exn_types.iter().collect();
                    let exn_ty = **input.choose(&types)?;
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let store = **input.choose(&stores)?;
                    let id = scope.next_id();
                    scope.exn_ref_pres.insert(id, store);
                    Ok(ExnRefPreNew { id, exn_ty, store })
                });
            }
            if swarm.exn_ref_pre_drop && !scope.exn_ref_pres.is_empty() {
                choices.push(|input, scope| {
                    let pres: Vec<_> = scope.exn_ref_pres.keys().collect();
                    let id = **input.choose(&pres)?;
                    scope.exn_ref_pres.remove(&id);
                    Ok(ExnRefPreDrop { id })
                });
            }
            if swarm.exn_ref_new
                && scope
                    .exn_ref_pres
                    .values()
                    .any(|sid| scope.tags.values().any(|tsid| tsid == sid))
            {
                choices.push(|input, scope| {
                    let pres_with_tags: Vec<_> = scope
                        .exn_ref_pres
                        .iter()
                        .filter(|&(_, &sid)| scope.tags.values().any(|&tsid| tsid == sid))
                        .collect();
                    let (&pre, &store_id) = *input.choose(&pres_with_tags)?;
                    let same_store_tags: Vec<_> = scope
                        .tags
                        .iter()
                        .filter(|&(_, &sid)| sid == store_id)
                        .collect();
                    let (&tag, _) = *input.choose(&same_store_tags)?;
                    let id = scope.next_id();
                    scope.exn_refs.insert(id, store_id);
                    Ok(ExnRefNew { id, pre, tag })
                });
            }
            if swarm.exn_ref_ty && !scope.exn_refs.is_empty() {
                choices.push(|input, scope| {
                    let refs: Vec<_> = scope.exn_refs.keys().collect();
                    let exn_ref = **input.choose(&refs)?;
                    Ok(ExnRefTy { exn_ref })
                });
            }
            if swarm.exn_ref_tag && !scope.exn_refs.is_empty() {
                choices.push(|input, scope| {
                    let refs: Vec<_> = scope.exn_refs.keys().collect();
                    let exn_ref = **input.choose(&refs)?;
                    Ok(ExnRefTag { exn_ref })
                });
            }
            if swarm.exn_ref_field && !scope.exn_refs.is_empty() {
                choices.push(|input, scope| {
                    let refs: Vec<_> = scope.exn_refs.keys().collect();
                    let exn_ref = **input.choose(&refs)?;
                    let index = usize::arbitrary(input)?;
                    Ok(ExnRefField { exn_ref, index })
                });
            }
            if swarm.exn_ref_drop && !scope.exn_refs.is_empty() {
                choices.push(|input, scope| {
                    let refs: Vec<_> = scope.exn_refs.keys().collect();
                    let id = **input.choose(&refs)?;
                    scope.exn_refs.remove(&id);
                    Ok(ExnRefDrop { id })
                });
            }

            if choices.is_empty() {
                break;
            }
            let c = input.choose(&choices)?;
            calls.push(c(input, &mut scope)?);
        }

        Ok(ApiCalls { calls })
    }
}
